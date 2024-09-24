use ariadne::Source;
use rowan::ast::AstNode;
use rowan::TextRange;

use crate::hir::*;
use crate::module_index::{Module, ModuleID};
use crate::report::{Report, ReportKind, Result};
use crate::{ast, cst::Token};

pub fn run(
    root: ast::Root,
    source: Source,
    module_id: ModuleID,
    parse_errors: Vec<Report>,
) -> Result<Module> {
    let mut lower = LowerCtx {
        module_id,
        spans: SpanMap::default(),
        errors: parse_errors,
        nodes: Default::default(),
    };
    let top_level = root.items().filter_map(|item| lower.item(item)).collect();
    let LowerCtx {
        spans,
        errors,
        nodes,
        ..
    } = lower;

    Ok(Module {
        source,
        nodes,
        spans,
        top_level,
        parser_errors: errors,
    })
}

struct LowerCtx {
    module_id: ModuleID,
    spans: SpanMap,
    errors: Vec<Report>,
    nodes: Arena,
}

impl LowerCtx {
    fn item(&mut self, item: ast::Item) -> Option<Idx> {
        let span = item.syntax().text_range();
        Some(match item {
            ast::Item::System(system) => return self.system(system),
            ast::Item::Resource(resource) => {
                let typed_ident = self.typed_ident(resource.field()?)?;
                self.alloc(span, Node::Resource(typed_ident))
            }
            ast::Item::Component(component) => {
                let typed_ident = self.typed_ident(component.field()?)?;
                self.alloc(span, Node::Component(typed_ident))
            }
            ast::Item::Import(import) => self.alloc(span, Node::Import(import.ident()?)),
        })
    }

    fn typed_ident(&mut self, field: ast::FieldType) -> Option<TypedIdent> {
        let ident = field.ident()?;
        let r#type = self.r#type(field.r#type()?)?;
        Some(TypedIdent { ident, r#type })
    }

    fn r#type(&mut self, r#type: ast::Type) -> Option<Idx> {
        let span = r#type.syntax().text_range();
        Some(match r#type {
            ast::Type::IdentifierType(ident) => {
                self.alloc(span, Node::TypeIdent(TypeIdent(ident.ident()?)))
            }
            ast::Type::StructType(r#struct) => {
                let fields = r#struct
                    .fields()
                    .filter_map(|field| self.typed_ident(field))
                    .collect();
                self.alloc(span, Node::Struct(StructType(fields)))
            }
        })
    }

    fn system(&mut self, system: ast::System) -> Option<Idx> {
        let block = self.expr(ast::Expr::Block(system.block()?));

        let id = self.nodes.alloc(Node::System(System {
            // _span: system.syntax().text_range(),
            ident: system.ident(),
            block,
        }));

        Some(id)
    }
}

mod expr {
    use super::*;

    impl LowerCtx {
        pub(super) fn expr(&mut self, expr: impl Into<Option<ast::Expr>>) -> Idx {
            let Some(expr) = expr.into() else {
                return self.alloc_expr_poison();
            };
            match expr {
                ast::Expr::UnaryExpr(unary_expr) => self.unary_expr(unary_expr),
                ast::Expr::BinaryExpr(binary_expr) => self.binary_expr(binary_expr),
                ast::Expr::Literal(literal) => {
                    if let Some(ident) = literal.as_identifier() {
                        self.alloc(
                            literal.syntax().text_range(),
                            Expr::Identifier(Identifier::Name(ident)),
                        )
                    } else if let Some(number) = literal.as_number() {
                        self.alloc(literal.syntax().text_range(), Expr::Number(number))
                    } else {
                        self.alloc(literal.syntax().text_range(), Expr::Poison)
                    }
                }
                ast::Expr::Query(query) => {
                    let Some(entity) = query.entity() else {
                        return self.alloc(query.syntax().text_range(), Expr::Poison);
                    };
                    let block = self.expr(query.block().map(ast::Expr::Block));
                    self.alloc(query.syntax().text_range(), Expr::Query { entity, block })
                }
                ast::Expr::Block(block) => self.block(block),
                ast::Expr::ECSAccess(access) => {
                    let span = access.syntax().text_range();
                    let component = self.expr(access.component());
                    if let Some(entity) = access.entity() {
                        let entity = self.expr(entity);
                        self.alloc(span, Expr::ComponentAccess { entity, component })
                    } else {
                        self.alloc(
                            span,
                            Expr::ResourceAccess {
                                resource: component,
                            },
                        )
                    }
                }
                // TODO: validate lvalue
                ast::Expr::Let(r#let) => {
                    let span = r#let.syntax().text_range();
                    let lvalue = self.expr(r#let.lvalue());
                    let expr = self.expr(r#let.expr());
                    self.alloc(span, Expr::Let { lvalue, expr })
                }
                // TODO: validate del value
                ast::Expr::Del(del) => {
                    let span = del.syntax().text_range();
                    let expr = self.expr(del.expr());
                    self.alloc(span, Expr::Del { expr })
                }
                ast::Expr::If(r#if) => {
                    let span = r#if.syntax().text_range();
                    let cond = self.expr(r#if.cond());
                    let expr = self.expr(r#if.expr());
                    let r#else = r#if.r#else().map(|e| self.expr(e));
                    self.alloc(span, Expr::If { cond, expr, r#else })
                }
                ast::Expr::While(r#while) => {
                    let span = r#while.syntax().text_range();
                    let cond = self.expr(r#while.cond());
                    let expr = self.expr(r#while.expr());
                    self.alloc(span, Expr::While { cond, expr })
                }
                ast::Expr::Call(call) => {
                    let span = call.syntax().text_range();
                    let function = self.expr(call.function());
                    let args = call.args().into_iter().map(|arg| self.expr(arg)).collect();

                    self.alloc(span, Expr::FunctionCall { function, args })
                }
                ast::Expr::Struct(r#struct) => {
                    let span = r#struct.syntax().text_range();
                    let Some(ident) = r#struct.ident() else {
                        return self.alloc_expr_poison();
                    };
                    let ident = Identifier::Name(ident);
                    let fields = r#struct
                        .fields()
                        .map(|(member, expr)| (member, self.expr(expr)))
                        .collect();
                    self.alloc(span, Expr::Struct { ident, fields })
                }
            }
        }

        fn block(&mut self, block: ast::Block) -> Idx {
            let exprs = block.exprs().map(|expr| self.expr(expr)).collect();
            self.alloc(block.syntax().text_range(), Expr::Block(exprs))
        }

        fn unary_expr(&mut self, unary_expr: ast::UnaryExpr) -> Idx {
            let span = unary_expr.syntax().text_range();
            let Some(op) = unary_expr.op() else {
                return self.alloc_expr_poison();
            };
            let op_ident = match op.kind() {
                Token::MINUS => Operator::Neg,
                Token::EXCLAMATION => Operator::Not,
                _ => unreachable!("This token shouldn't be parsed"),
            };

            let op = self.alloc(
                op.text_range(),
                Expr::Identifier(Identifier::Operator(op_ident)),
            );

            let expr = Expr::FunctionCall {
                function: op,
                args: vec![self.expr(unary_expr.val())],
            };

            self.alloc(span, expr)
        }

        fn binary_expr(&mut self, binary_expr: ast::BinaryExpr) -> Idx {
            let span = binary_expr.syntax().text_range();

            let left = self.expr(binary_expr.left());
            let right = self.expr(binary_expr.right());

            let Some(op) = binary_expr.op() else {
                return self.alloc_expr_poison();
            };

            let op_kind = match op.kind() {
                Token::PLUS => Operator::Add,
                Token::MINUS => Operator::Sub,
                Token::STAR => Operator::Mul,
                Token::SLASH => Operator::Div,
                Token::EQEQ => Operator::Eq,
                Token::LESS => Operator::Less,
                Token::LESSEQ => Operator::LessEq,
                Token::GREATER => Operator::Greater,
                Token::GREATEREQ => Operator::GreaterEq,
                Token::OROR => Operator::Or,
                Token::ANDAND => Operator::And,
                Token::DOT => {
                    return self.alloc(
                        span,
                        Expr::MemberAccess {
                            base: left,
                            member: right,
                        },
                    )
                }
                Token::EQ => {
                    // TODO: validate lvalue
                    return self.alloc(
                        span,
                        Expr::Assign {
                            lvalue: left,
                            expr: right,
                        },
                    );
                }
                Token::PLUSEQ => {
                    return self.op_assign(span, left, right, op.text_range(), Operator::Add);
                }
                Token::MINUSEQ => {
                    return self.op_assign(span, left, right, op.text_range(), Operator::Sub);
                }
                Token::STAREQ => {
                    return self.op_assign(span, left, right, op.text_range(), Operator::Mul);
                }
                Token::SLASHEQ => {
                    return self.op_assign(span, left, right, op.text_range(), Operator::Div);
                }
                _ => unreachable!("Token shouldn't be parsed as an operator"),
            };

            let op = self.alloc(
                op.text_range(),
                Expr::Identifier(Identifier::Operator(op_kind)),
            );

            let expr = Expr::FunctionCall {
                function: op,
                args: vec![left, right],
            };

            self.alloc(span, expr)
        }

        fn op_assign(
            &mut self,
            span: TextRange,
            left: Idx,
            right: Idx,
            op_span: TextRange,
            op: Operator,
        ) -> Idx {
            let function = self.alloc(op_span, Expr::Identifier(Identifier::Operator(op)));
            let expr = self.alloc(
                span,
                Expr::FunctionCall {
                    function,
                    args: vec![left, right],
                },
            );
            self.alloc(span, Expr::Assign { lvalue: left, expr })
        }
    }

    impl LowerCtx {
        fn alloc_expr_poison(&mut self) -> Idx {
            self.errors
                .push(Report::build(ReportKind::Error, self.module_id, 0).finish());
            self.nodes.alloc(Node::Expr(Expr::Poison))
        }
    }
}

impl LowerCtx {
    fn alloc(&mut self, span: impl Into<TextRange>, expr: impl Into<Node>) -> Idx {
        let idx = self.nodes.alloc(expr.into());
        self.spans.insert(idx, span.into());
        idx
    }
}

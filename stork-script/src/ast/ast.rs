use crate::cst::{StorkLang, SyntaxNode, SyntaxToken, Token};
use crate::module_index::ModuleID;
use crate::report::{Report, Result, INTERNAL_REPORT_KIND};
use rowan::{ast::AstNode, GreenNode, SyntaxElement, SyntaxElementChildren};
use std::fmt::{Debug, DebugTuple};

pub fn run(cst: GreenNode, module_id: ModuleID) -> Result<Root> {
    let node = SyntaxNode::new_root(cst);
    Root::cast(node)
        .ok_or_else(|| Box::new(Report::build(INTERNAL_REPORT_KIND, module_id, 0).finish()))
}

trait DebugTupleEx {
    fn option_field<D: Debug>(&mut self, f: &Option<D>) -> &mut Self;
}

impl DebugTupleEx for DebugTuple<'_, '_> {
    fn option_field<D: Debug>(&mut self, f: &Option<D>) -> &mut Self {
        match f {
            Some(f) => self.field(f),
            _ => self.field(&"Option::None"),
        };
        self
    }
}

macro_rules! ast {
    (struct $Self:ident => $token:expr) => {
        pub struct $Self(SyntaxNode);

        impl AstNode for $Self {
            type Language = StorkLang;

            fn can_cast(kind: Token) -> bool
            {
                kind == $token
            }

            fn cast(node: SyntaxNode) -> Option<Self>
            {
                Self::can_cast(node.kind()).then_some($Self(node))
            }

            fn syntax(&self) -> &SyntaxNode {
                &self.0
            }
        }
    };
    (enum $Self:ident {
        $($Variant:ident,)*
    }) => {
        pub enum $Self {
            $($Variant($Variant),)*
        }

        impl AstNode for $Self {
            type Language = StorkLang;

            fn can_cast(kind: Token) -> bool
            {
                $($Variant::can_cast(kind))||*
            }

            fn cast(node: SyntaxNode) -> Option<Self>
            {
                $(
                if $Variant::can_cast(node.kind()) {
                    Some($Self::$Variant($Variant::cast(node)?))
                } else
                )* {
                    None
                }
            }

            fn syntax(&self) -> &SyntaxNode {
                match self {
                    $($Self::$Variant(i) => i.syntax(),)*
                }
            }
        }

        impl Debug for $Self {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $($Self::$Variant(i) => Debug::fmt(&i, f),)*
                }
            }
        }
    };
}

ast!(struct Root => Token::Root);
impl Root {
    pub fn items(&self) -> impl Iterator<Item = Item> {
        self.0.children().filter_map(Item::cast)
    }
}
impl Debug for Root {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_tuple(&format!("Root @{:?}", self.0.text_range()));
        for expr in self.items() {
            f.field(&expr);
        }
        f.finish()
    }
}

ast!(
    enum Item {
        System,
        Resource,
        Component,
        Import,
    }
);
ast!(struct System => Token::System);
impl System {
    pub fn ident(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|t| t.kind() == Token::IDENT)
            .map(|s| s.text().to_string())
    }

    pub fn block(&self) -> Option<Block> {
        self.0.children().find_map(Block::cast)
    }
}
impl Debug for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(&format!("System @{:?}", self.0.text_range()))
            .option_field(&self.ident())
            .option_field(&self.block())
            .finish()
    }
}

ast!(struct Component => Token::Component);
impl Component {
    pub fn field(&self) -> Option<FieldType> {
        self.0.children().find_map(FieldType::cast)
    }
}
impl Debug for Component {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(&format!("Component @{:?}", self.0.text_range()))
            .option_field(&self.field())
            .finish()
    }
}

ast!(struct Resource => Token::Resource);
impl Resource {
    pub fn field(&self) -> Option<FieldType> {
        self.0.children().find_map(FieldType::cast)
    }
}
impl Debug for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(&format!("Resource @{:?}", self.0.text_range()))
            .option_field(&self.field())
            .finish()
    }
}
ast!(struct Import => Token::Import);
impl Import {
    pub fn ident(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|t| t.kind() == Token::IDENT)
            .map(|s| s.text().to_string())
    }
}
impl Debug for Import {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(&format!("Import @{:?}", self.0.text_range()))
            .option_field(&self.ident())
            .finish()
    }
}

ast!(struct FieldType => Token::FieldType);
impl FieldType {
    pub fn ident(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|t| t.kind() == Token::IDENT)
            .map(|s| s.text().to_string())
    }

    pub fn r#type(&self) -> Option<Type> {
        self.0.children().find_map(Type::cast)
    }
}
impl Debug for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(&format!("Field @{:?}", self.0.text_range()))
            .option_field(&self.ident())
            .option_field(&self.r#type())
            .finish()
    }
}

ast!(
    enum Type {
        IdentifierType,
        StructType,
    }
);
impl Type {}

ast!(struct IdentifierType => Token::Literal);
impl IdentifierType {
    pub fn ident(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|t| t.kind() == Token::IDENT)
            .map(|s| s.text().to_string())
    }
}
impl Debug for IdentifierType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(&format!("IdentifierType @{:?}", self.0.text_range()))
            .option_field(&self.ident())
            .finish()
    }
}

ast!(struct StructType => Token::StructType);
impl StructType {
    pub fn fields(&self) -> impl Iterator<Item = FieldType> {
        self.0.children().filter_map(FieldType::cast)
    }
}
impl Debug for StructType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_tuple(&format!("StructType @{:?}", self.0.text_range()));
        for expr in self.fields() {
            f.field(&expr);
        }
        f.finish()
    }
}

ast!(
    enum Expr {
        UnaryExpr,
        BinaryExpr,
        Literal,
        Query,
        Block,
        ECSAccess,
        Let,
        Del,
        If,
        While,
        Call,
        Struct,
    }
);

ast!(struct Let => Token::Let);
impl Let {
    pub fn lvalue(&self) -> Option<Expr> {
        self.0.children().filter_map(Expr::cast).nth(0)
    }

    pub fn expr(&self) -> Option<Expr> {
        self.0.children().filter_map(Expr::cast).nth(1)
    }
}
impl Debug for Let {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(&format!("Let @{:?}", self.0.text_range()))
            .option_field(&self.lvalue())
            .option_field(&self.expr())
            .finish()
    }
}

ast!(struct Del => Token::Del);
impl Del {
    pub fn expr(&self) -> Option<Expr> {
        self.0.children().filter_map(Expr::cast).nth(0)
    }
}
impl Debug for Del {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(&format!("Del @{:?}", self.0.text_range()))
            .option_field(&self.expr())
            .finish()
    }
}

ast!(struct If => Token::If);
impl If {
    pub fn has_else(&self) -> bool {
        self.0
            .children_with_tokens()
            .any(|t| t.kind() == Token::ELSE)
    }

    pub fn cond(&self) -> Option<Expr> {
        self.0.children().filter_map(Expr::cast).nth(0)
    }

    pub fn expr(&self) -> Option<Expr> {
        self.0.children().filter_map(Expr::cast).nth(1)
    }

    pub fn r#else(&self) -> Option<Option<Expr>> {
        self.has_else()
            .then_some(self.0.children().filter_map(Expr::cast).nth(2))
    }
}
impl Debug for If {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(&format!("If @{:?}", self.0.text_range()))
            .option_field(&self.cond())
            .option_field(&self.expr())
            .option_field(&self.r#else())
            .finish()
    }
}

ast!(struct While => Token::While);
impl While {
    pub fn cond(&self) -> Option<Expr> {
        self.0.children().filter_map(Expr::cast).nth(0)
    }

    pub fn expr(&self) -> Option<Expr> {
        self.0.children().filter_map(Expr::cast).nth(1)
    }
}
impl Debug for While {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(&format!("While @{:?}", self.0.text_range()))
            .option_field(&self.cond())
            .option_field(&self.expr())
            .finish()
    }
}

ast!(struct Call => Token::Call);
impl Call {
    pub fn function(&self) -> Option<Expr> {
        self.0.children().filter_map(Expr::cast).nth(0)
    }

    pub fn args(&self) -> Vec<Expr> {
        self.0.children().filter_map(Expr::cast).skip(1).collect()
    }
}
impl Debug for Call {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(&format!("Call @{:?}", self.0.text_range()))
            .option_field(&self.function())
            .field(&self.args())
            .finish()
    }
}

ast!(struct UnaryExpr => Token::Prefix);
impl UnaryExpr {
    pub fn val(&self) -> Option<Expr> {
        self.0.children().filter_map(Expr::cast).nth(0)
    }

    pub fn op(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|t| t.kind().is_prefix_op())
    }
}
impl Debug for UnaryExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(&format!("UnaryExpr @{:?}", self.0.text_range()))
            .option_field(&self.op())
            .option_field(&self.val())
            .finish()
    }
}

ast!(struct BinaryExpr => Token::Infix);
impl BinaryExpr {
    pub fn left(&self) -> Option<Expr> {
        self.0.children().filter_map(Expr::cast).nth(0)
    }

    pub fn right(&self) -> Option<Expr> {
        self.0.children().filter_map(Expr::cast).nth(1)
    }

    pub fn op(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|t| t.kind().is_infix_op())
    }
}
impl Debug for BinaryExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(&format!("BinaryExpr @{:?}", self.0.text_range()))
            .option_field(&self.left())
            .option_field(&self.op())
            .option_field(&self.right())
            .finish()
    }
}

pub struct ECSAccess(SyntaxNode);
impl AstNode for ECSAccess {
    type Language = StorkLang;

    fn can_cast(kind: Token) -> bool {
        kind == Token::ComponentAccess || kind == Token::ResourceAccess
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        Self::can_cast(node.kind()).then_some(ECSAccess(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}
impl ECSAccess {
    pub fn is_component(&self) -> bool {
        self.syntax().kind() == Token::ComponentAccess
    }

    pub fn entity(&self) -> Option<Option<Expr>> {
        self.is_component()
            .then_some(self.0.children().filter_map(Expr::cast).nth(0))
    }

    pub fn component(&self) -> Option<Expr> {
        self.0
            .children()
            .filter_map(Expr::cast)
            .nth(if self.is_component() { 1 } else { 0 })
    }
}
impl Debug for ECSAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(&format!("ECSAccess @{:?}", self.0.text_range()))
            .option_field(&self.entity())
            .option_field(&self.component())
            .finish()
    }
}

ast!(struct Literal => Token::Literal);
impl Literal {
    pub fn as_number(&self) -> Option<f32> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|t| t.kind() == Token::NUMBER)?
            .text()
            .parse()
            .ok()
    }

    pub fn as_identifier(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|t| t.kind() == Token::IDENT)
            .map(|s| s.text().to_string())
    }
}
impl Debug for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(&format!("Literal @{:?}", self.0.text_range()))
            .field(&self.0.text())
            .finish()
    }
}

ast!(struct Query => Token::Query);
impl Query {
    pub fn entity(&self) -> Option<String> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|t| t.kind() == Token::IDENT)
            .map(|s| s.text().to_string())
    }

    pub fn block(&self) -> Option<Block> {
        self.0.children().find_map(Block::cast)
    }
}
impl Debug for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(&format!("Query @{:?}", self.0.text_range()))
            .option_field(&self.block())
            .finish()
    }
}

ast!(struct Block => Token::Block);
impl Block {
    pub fn exprs(&self) -> impl Iterator<Item = Expr> {
        self.0.children().filter_map(Expr::cast)
    }
}
impl Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_tuple(&format!("Block @{:?}", self.0.text_range()));
        for expr in self.exprs() {
            f.field(&expr);
        }
        f.finish()
    }
}

ast!(struct Struct => Token::Struct);
impl Struct {
    pub fn ident(&self) -> Option<String> {
        self.0
            .children()
            .find_map(Literal::cast)
            .and_then(|l| l.as_identifier())
    }

    pub fn fields(&self) -> impl Iterator<Item = (String, Expr)> {
        struct Iter(SyntaxElementChildren<StorkLang>);

        impl Iterator for Iter {
            type Item = (String, Expr);

            fn next(&mut self) -> Option<Self::Item> {
                let ident = loop {
                    if let Some(t) = self.0.next()?.into_token() {
                        if t.kind() == Token::IDENT {
                            break t.text().to_string();
                        }
                    }
                };
                let expr = loop {
                    if let Some(t) = self.0.next()?.into_node() {
                        if let Some(expr) = Expr::cast(t) {
                            break expr;
                        }
                    }
                };
                Some((ident, expr))
            }
        }

        Iter(self.0.children_with_tokens())
    }
}
impl Debug for Struct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut f = f.debug_tuple(&format!("Struct @{:?}", self.0.text_range()));
        f.option_field(&self.ident());
        for expr in self.fields() {
            f.field(&expr);
        }
        f.finish()
    }
}

#[cfg(test)]
mod test;

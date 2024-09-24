use ariadne::ReportKind;
use logos::{Logos, Source};
use rowan::{Checkpoint, GreenNode, GreenNodeBuilder, Language, SyntaxKind};
use std::ops::Range;

use crate::{
    module_index::ModuleID,
    report::{Label, Report, ReportBuilder, Result, INTERNAL_REPORT_KIND},
};

pub fn run(source: &str, module_id: ModuleID) -> Result<(GreenNode, Vec<Report>)> {
    let mut errors = Vec::new();
    let parser = Parser::new(source, module_id, &mut errors);
    Ok((parser.parse()?, errors))
}

pub struct ParseCtx<'e> {
    module_id: ModuleID,
    errors: &'e mut Vec<Report>,
}

#[allow(non_camel_case_types)]
#[derive(Logos, Debug, PartialEq, Clone, Copy, Hash, Eq, PartialOrd, Ord)]
#[logos(extras = ParseCtx<'s>)]
#[repr(u16)]
pub enum Token {
    // Keywords
    #[token("comp")]
    COMP,
    #[token("res")]
    RES,
    #[token("sys")]
    SYS,
    #[token("query")]
    QUERY,
    #[token("fn")]
    FN,
    #[token("if")]
    IF,
    #[token("else")]
    ELSE,
    #[token("while")]
    WHILE,
    #[token("let")]
    LET,
    #[token("del")]
    DEL,
    #[token("use")]
    USE,

    // Whitespace
    #[regex(r"[ \t]+")]
    WHITE_SPACE,
    #[regex(r"\n+")]
    NEW_LINE,

    // Comment
    #[regex(r"#.*\n")]
    COMMENT,

    // User
    #[regex(r"(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?")]
    NUMBER,
    #[regex(r"[_a-zA-Z][0-9a-zA-Z_]*")]
    IDENT,

    // "Operators"
    #[token("+")]
    PLUS,
    #[token("+=")]
    PLUSEQ,
    #[token("-")]
    MINUS,
    #[token("-=")]
    MINUSEQ,
    #[token("*")]
    STAR,
    #[token("*=")]
    STAREQ,
    #[token("/")]
    SLASH,
    #[token("/=")]
    SLASHEQ,
    #[token(":")]
    COLON,
    #[token("{")]
    LBRACE,
    #[token("}")]
    RBRACE,
    #[token("[")]
    LBRACKET,
    #[token("]")]
    RBRACKET,
    #[token("(")]
    LPAREN,
    #[token(")")]
    RPAREN,
    #[token(".")]
    DOT,
    #[token("=")]
    EQ,
    #[token(";")]
    SEMICOLON,
    #[token(",")]
    COMMA,
    #[token("==")]
    EQEQ,
    #[token("<")]
    LESS,
    #[token("<=")]
    LESSEQ,
    #[token(">")]
    GREATER,
    #[token(">=")]
    GREATEREQ,
    #[token("!")]
    EXCLAMATION,
    #[token("||")]
    OROR,
    #[token("&&")]
    ANDAND,

    // Composite
    Error,
    Root,
    Resource,
    Component,
    System,
    Function,
    Import,

    // Composite, types
    FieldType,
    StructType,

    // Composite, exprs
    Paren,
    Query,
    Block,
    Prefix,
    Infix,
    Literal,
    ComponentAccess,
    ResourceAccess,
    Call,
    Let,
    Del,
    While,
    If,
    Struct,

    // Other
    UNKNOWN,
    EOF,
}

impl Token {
    fn infix_binding_power(self) -> Option<(u8, u8)> {
        use Token::*;
        Some(match self {
            EQ | PLUSEQ | MINUSEQ | STAREQ | SLASHEQ | OROR | ANDAND => (2, 1),
            EQEQ | GREATER | GREATEREQ | LESS | LESSEQ => (3, 4),
            PLUS | MINUS | EXCLAMATION => (5, 6),
            STAR | SLASH => (7, 8),
            DOT | LBRACKET | LPAREN => (9, 10),
            _ => return None,
        })
    }

    pub fn is_infix_op(self) -> bool {
        self.infix_binding_power().is_some()
    }

    fn prefix_binding_power(self) -> Option<u8> {
        use Token::*;
        Some(match self {
            PLUS | MINUS | EXCLAMATION | LBRACKET => 8,
            _ => return None,
        })
    }

    pub fn is_prefix_op(self) -> bool {
        self.prefix_binding_power().is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StorkLang {}
impl rowan::Language for StorkLang {
    type Kind = Token;
    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        assert!(raw.0 <= Token::EOF as u16);
        unsafe { std::mem::transmute::<u16, Token>(raw.0) }
    }
    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}

impl From<Token> for rowan::SyntaxKind {
    fn from(kind: Token) -> Self {
        StorkLang::kind_to_raw(kind)
    }
}

impl From<rowan::SyntaxKind> for Token {
    fn from(raw: rowan::SyntaxKind) -> Self {
        StorkLang::kind_from_raw(raw)
    }
}

pub struct Parser<'a> {
    builder: GreenNodeBuilder<'a>,
    source: &'a str,
    iter: logos::Lexer<'a, Token>,
    token: Token,
    span: Range<usize>,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str, module_id: ModuleID, errors: &'a mut Vec<Report>) -> Self {
        let mut iter = Token::lexer_with_extras(source, ParseCtx { module_id, errors });
        let token = iter
            .next()
            .unwrap_or(Ok(Token::EOF))
            .unwrap_or(Token::UNKNOWN);
        let span = iter.span();
        Self {
            builder: GreenNodeBuilder::new(),
            source,
            iter,
            token,
            span,
        }
    }

    fn kind(&self) -> SyntaxKind {
        self.token.into()
    }

    fn span(&self) -> Range<usize> {
        self.span.clone()
    }

    fn report(&self) -> ReportBuilder {
        Report::build(
            ReportKind::Error,
            self.iter.extras.module_id,
            self.span.start,
        )
    }

    fn report_internal(&self) -> ReportBuilder {
        Report::build(
            INTERNAL_REPORT_KIND,
            self.iter.extras.module_id,
            self.span.start,
        )
    }

    fn label<M: ToString>(&self, m: M) -> Label {
        Label::new((self.iter.extras.module_id, self.span.clone())).with_message(m)
    }

    fn text(&self) -> Result<&'a str> {
        self.source.slice(self.span()).ok_or_else(|| {
            Box::new(
                self.report_internal()
                    .with_message("Span should always be inside source")
                    .finish(),
            )
        })
    }

    fn bump(&mut self) -> Result<()> {
        self.builder.token(self.kind(), self.text()?);
        self.token = self
            .iter
            .next()
            .unwrap_or(Ok(Token::EOF))
            .unwrap_or(Token::UNKNOWN);
        self.span = self.iter.span();
        Ok(())
    }

    fn eat_ws(&mut self) -> Result<()> {
        while self.token == Token::WHITE_SPACE
            || self.token == Token::NEW_LINE
            || self.token == Token::COMMENT
        {
            self.bump()?;
        }
        Ok(())
    }

    #[track_caller]
    fn expect(&mut self, token: Token) -> Result<()> {
        let mut error = None;
        while self.token != token && self.token != Token::EOF {
            error = error.or(Some(
                self.report()
                    .with_label(self.label("here"))
                    .with_message("Unexpected token")
                    .with_help(format!("Expected {token:?}"))
                    .finish(),
            ));
            self.bump()?;
        }
        if let Some(error) = error {
            self.iter.extras.errors.push(error);
        }
        Ok(())
    }

    fn leaf(&mut self, token: Token) -> Result<()> {
        self.builder.start_node(token.into());
        self.bump()?;
        self.builder.finish_node();
        Ok(())
    }

    fn node(&mut self, token: Token, f: impl FnOnce(&mut Self) -> Result<()>) -> Result<()> {
        self.builder.start_node(token.into());
        self.bump()?;
        self.eat_ws()?;
        let r = f(self);
        self.builder.finish_node();
        r
    }

    fn checkpoint(&mut self) -> Checkpoint {
        self.builder.checkpoint()
    }

    fn checkpoint_node(
        &mut self,
        checkpoint: Checkpoint,
        token: Token,
        f: impl FnOnce(&mut Self) -> Result<()>,
    ) -> Result<()> {
        self.builder.start_node_at(checkpoint, token.into());
        self.bump()?;
        self.eat_ws()?;
        let r = f(self);
        self.builder.finish_node();
        r
    }

    pub fn parse(mut self) -> Result<GreenNode> {
        self.builder.start_node(Token::Root.into());
        self.eat_ws()?;
        while self.token != Token::EOF {
            self.parse_item()?;
            self.eat_ws()?;
        }
        self.builder.finish_node();
        Ok(self.builder.finish())
    }

    fn parse_item(&mut self) -> Result<()> {
        match self.token {
            Token::COMP => self.parse_component(),
            Token::RES => self.parse_resource(),
            Token::SYS => self.parse_system(),
            Token::USE => self.parse_import(),
            _ => self.leaf(Token::Error),
        }
    }

    fn parse_component(&mut self) -> Result<()> {
        self.node(Token::Component, |s| s.parse_field_def())
    }

    fn parse_resource(&mut self) -> Result<()> {
        self.node(Token::Resource, |s| s.parse_field_def())
    }

    fn parse_system(&mut self) -> Result<()> {
        self.node(Token::System, |s| {
            if s.token == Token::IDENT {
                s.bump()?;
                s.eat_ws()?;
            }
            s.expect(Token::LBRACE)?;
            s.parse_block()
        })
    }

    fn parse_import(&mut self) -> Result<()> {
        self.node(Token::Import, |s| {
            s.expect(Token::IDENT)?;
            s.bump()
        })
    }

    fn parse_block(&mut self) -> Result<()> {
        self.node(Token::Block, |s| {
            while s.token != Token::RBRACE {
                let parsed_block = s.parse_expr(None)?;
                s.eat_ws()?;
                if s.token == Token::SEMICOLON {
                    s.bump()?;
                    s.eat_ws()?;
                } else if parsed_block == ParsedBlock::No {
                    s.expect(Token::RBRACE)?;
                }
            }
            s.bump()
        })
    }

    fn parse_expr(&mut self, min_bp: impl Into<Option<u8>>) -> Result<ParsedBlock> {
        let checkpoint = self.checkpoint();
        let mut parsed_block = ParsedBlock::No;

        match self.token {
            Token::NUMBER => self.leaf(Token::Literal)?,
            Token::IDENT => {
                let struct_checkpoint = self.checkpoint();
                self.leaf(Token::Literal)?;

                self.eat_ws()?;
                if self.token == Token::LBRACE {
                    self.checkpoint_node(struct_checkpoint, Token::Struct, |s| {
                        while s.token != Token::RBRACE {
                            s.expect(Token::IDENT)?;
                            s.bump()?;
                            s.eat_ws()?;
                            s.expect(Token::COLON)?;
                            s.bump()?;
                            s.eat_ws()?;
                            s.parse_expr(None)?;
                            if s.token == Token::COMMA {
                                s.bump()?;
                                s.eat_ws()?;
                            } else {
                                s.expect(Token::RBRACE)?;
                            }
                        }
                        s.bump()
                    })?;
                }
            }
            Token::QUERY => {
                parsed_block = ParsedBlock::Yes;
                self.parse_query()?;
            }
            Token::LPAREN => {
                self.node(Token::Paren, |s| {
                    s.parse_expr(None)?;
                    s.expect(Token::RPAREN)?;
                    s.bump()
                })?;
            }
            Token::LBRACE => {
                parsed_block = ParsedBlock::Yes;
                self.parse_block()?;
            }
            Token::DEL => {
                self.node(Token::Del, |s| {
                    s.parse_expr(None)?;
                    Ok(())
                })?;
            }
            Token::IF => {
                parsed_block = ParsedBlock::Yes;
                self.node(Token::If, |s| {
                    s.parse_expr(None)?;
                    s.eat_ws()?;
                    s.parse_block()?;
                    // This should be done conditionally, depending if there is an else...
                    s.eat_ws()?;
                    if s.token == Token::ELSE {
                        s.bump()?;
                        s.eat_ws()?;
                        s.parse_block()?;
                    }
                    Ok(())
                })?;
            }
            Token::WHILE => {
                parsed_block = ParsedBlock::Yes;
                self.node(Token::While, |s| {
                    s.parse_expr(None)?;
                    s.eat_ws()?;
                    s.parse_block()
                })?;
            }
            Token::LET => {
                self.node(Token::Let, |s| {
                    let (l_bp, _) = Token::EQ.infix_binding_power().ok_or_else(|| {
                        s.report_internal()
                            .with_message("Token::EQ is an infix token")
                            .finish()
                    })?;
                    s.parse_expr(l_bp + 1)?;
                    s.expect(Token::EQ)?;
                    s.bump()?;
                    s.eat_ws()?;
                    s.parse_expr(None)?;
                    Ok(())
                })?;
            }
            Token::LBRACKET => {
                self.node(Token::ResourceAccess, |s| {
                    s.parse_expr(None)?;
                    s.expect(Token::RBRACKET)?;
                    s.bump()
                })?;
            }
            token if token.is_prefix_op() => {
                let bp = token.prefix_binding_power().ok_or_else(|| {
                    self.report_internal()
                        .with_message("Token is a prefix token")
                        .finish()
                })?;
                self.node(Token::Prefix, |s| {
                    s.parse_expr(bp)?;
                    Ok(())
                })?;
            }
            _ => {
                self.iter.extras.errors.push(
                    self.report()
                        .with_label(self.label("here"))
                        .with_message("Unexpected token")
                        .finish(),
                );
            }
        }

        let min_bp: u8 = min_bp.into().unwrap_or_default();

        loop {
            self.eat_ws()?;
            let Some((l_bp, r_bp)) = self.token.infix_binding_power() else {
                return Ok(parsed_block);
            };

            if l_bp < min_bp {
                return Ok(parsed_block);
            }

            parsed_block = ParsedBlock::No;
            if self.token == Token::LBRACKET {
                self.checkpoint_node(checkpoint, Token::ComponentAccess, |s| {
                    s.parse_expr(None)?;
                    s.expect(Token::RBRACKET)?;
                    s.bump()
                })?;
            } else if self.token == Token::LPAREN {
                self.checkpoint_node(checkpoint, Token::Call, |s| {
                    while s.token != Token::RPAREN {
                        s.parse_expr(None)?;
                        s.eat_ws()?;
                        if s.token == Token::COMMA {
                            s.bump()?;
                            s.eat_ws()?;
                        } else {
                            s.expect(Token::RPAREN)?;
                        }
                    }
                    s.bump()
                })?;
            } else {
                self.checkpoint_node(checkpoint, Token::Infix, |s| {
                    s.parse_expr(r_bp)?;
                    Ok(())
                })?;
            }
        }
    }

    fn parse_query(&mut self) -> Result<()> {
        self.node(Token::Query, |s| {
            if s.token == Token::IDENT {
                s.bump()?;
                s.eat_ws()?;
            }
            s.expect(Token::LBRACE)?;
            s.parse_block()
        })?;
        Ok(())
    }

    fn parse_struct_def(&mut self) -> Result<()> {
        self.node(Token::StructType, |s| {
            while s.token != Token::RBRACE {
                s.parse_field_def()?;
                s.eat_ws()?;
                if s.token == Token::COMMA {
                    s.bump()?;
                    s.eat_ws()?;
                } else {
                    s.expect(Token::RBRACE)?;
                }
            }
            s.bump()
        })?;
        Ok(())
    }

    fn parse_field_def(&mut self) -> Result<()> {
        self.expect(Token::IDENT)?;
        self.node(Token::FieldType, |s| {
            if s.token != Token::COLON {
                return Ok(());
            }
            s.bump()?;
            s.eat_ws()?;
            if s.token == Token::IDENT {
                s.leaf(Token::Literal)?;
            } else if s.token == Token::LBRACE {
                s.parse_struct_def()?;
            }
            Ok(())
        })?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ParsedBlock {
    Yes,
    No,
}

pub type SyntaxNode = rowan::SyntaxNode<StorkLang>;
pub type SyntaxToken = rowan::SyntaxToken<StorkLang>;
pub type SyntaxElement = rowan::SyntaxElement<StorkLang>;

#[cfg(test)]
mod test;

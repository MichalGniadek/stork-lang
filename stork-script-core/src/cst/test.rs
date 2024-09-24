use super::{Parser, SyntaxElement, Token};
use crate::cst::SyntaxNode;
use expect_test::{expect, Expect};
use itertools::intersperse;

fn print(node: &SyntaxElement, padding: usize, filter: &[Token]) -> String {
    let n = format!(
        "{}{:?} @{:?}\n{}",
        " ".repeat(padding),
        node.kind(),
        node.text_range(),
        node.as_node().map_or(String::new(), |node| intersperse(
            node.children_with_tokens()
                .filter(|c| !filter.contains(&c.kind()))
                .map(|c| print(&c, padding + 5, filter)),
            String::new()
        )
        .collect::<String>())
    );
    n
}

fn check(source: &str, expect: Expect) {
    let mut errors = Vec::new();
    let parser = Parser::new(source, 0, &mut errors);
    let result = parser.parse().unwrap();
    assert!(errors.is_empty());
    assert_eq!(result.to_string(), source);

    let node = SyntaxNode::new_root(result.clone());
    let tokens = print(&node.into(), 0, &[Token::WHITE_SPACE, Token::NEW_LINE]);
    expect.assert_eq(&tokens);
}

#[test]
fn test1() {
    check(
        "
        sys { 
            -1+2*(-7+3);
            asd.asd.asd
        }
        ",
        expect![[r#"
            Root @0..83
                 System @9..74
                      SYS @9..12
                      Block @13..74
                           LBRACE @13..14
                           Infix @28..39
                                Prefix @28..30
                                     MINUS @28..29
                                     Literal @29..30
                                          NUMBER @29..30
                                PLUS @30..31
                                Infix @31..39
                                     Literal @31..32
                                          NUMBER @31..32
                                     STAR @32..33
                                     Paren @33..39
                                          LPAREN @33..34
                                          Infix @34..38
                                               Prefix @34..36
                                                    MINUS @34..35
                                                    Literal @35..36
                                                         NUMBER @35..36
                                               PLUS @36..37
                                               Literal @37..38
                                                    NUMBER @37..38
                                          RPAREN @38..39
                           SEMICOLON @39..40
                           Infix @53..73
                                Infix @53..60
                                     Literal @53..56
                                          IDENT @53..56
                                     DOT @56..57
                                     Literal @57..60
                                          IDENT @57..60
                                DOT @60..61
                                Literal @61..64
                                     IDENT @61..64
                           RBRACE @73..74
        "#]],
    );
}
#[test]
fn test2() {
    check(
        "sys { query { Transform.position.x = 0.2; } }",
        expect![[r#"
            Root @0..45
                 System @0..45
                      SYS @0..3
                      Block @4..45
                           LBRACE @4..5
                           Query @6..43
                                QUERY @6..11
                                Block @12..43
                                     LBRACE @12..13
                                     Infix @14..40
                                          Infix @14..35
                                               Infix @14..32
                                                    Literal @14..23
                                                         IDENT @14..23
                                                    DOT @23..24
                                                    Literal @24..32
                                                         IDENT @24..32
                                               DOT @32..33
                                               Literal @33..34
                                                    IDENT @33..34
                                          EQ @35..36
                                          Literal @37..40
                                               NUMBER @37..40
                                     SEMICOLON @40..41
                                     RBRACE @42..43
                           RBRACE @44..45
        "#]],
    );
}

#[test]
fn test3() {
    check(
        "sys { 2 = {3} = asd }",
        expect![[r#"
            Root @0..21
                 System @0..21
                      SYS @0..3
                      Block @4..21
                           LBRACE @4..5
                           Infix @6..20
                                Literal @6..7
                                     NUMBER @6..7
                                EQ @8..9
                                Infix @10..20
                                     Block @10..13
                                          LBRACE @10..11
                                          Literal @11..12
                                               NUMBER @11..12
                                          RBRACE @12..13
                                     EQ @14..15
                                     Literal @16..19
                                          IDENT @16..19
                           RBRACE @20..21
        "#]],
    );
}

#[test]
fn test4() {
    check(
        "sys { 
          [C1].x;
          entity[C2].x
        }",
        expect![[r#"
            Root @0..57
                 System @0..57
                      SYS @0..3
                      Block @4..57
                           LBRACE @4..5
                           Infix @17..23
                                ResourceAccess @17..21
                                     LBRACKET @17..18
                                     Literal @18..20
                                          IDENT @18..20
                                     RBRACKET @20..21
                                DOT @21..22
                                Literal @22..23
                                     IDENT @22..23
                           SEMICOLON @23..24
                           Infix @35..56
                                ComponentAccess @35..45
                                     Literal @35..41
                                          IDENT @35..41
                                     LBRACKET @41..42
                                     Literal @42..44
                                          IDENT @42..44
                                     RBRACKET @44..45
                                DOT @45..46
                                Literal @46..47
                                     IDENT @46..47
                           RBRACE @56..57
        "#]],
    );
}

#[test]
fn test5() {
    check(
        "sys sys_name {
          query entity {}
        }",
        expect![[r#"
            Root @0..50
                 System @0..50
                      SYS @0..3
                      IDENT @4..12
                      Block @13..50
                           LBRACE @13..14
                           Query @25..40
                                QUERY @25..30
                                IDENT @31..37
                                Block @38..40
                                     LBRACE @38..39
                                     RBRACE @39..40
                           RBRACE @49..50
        "#]],
    );
}

#[test]
fn test6() {
    check(
        "
          res A: f32
          res B: {
               f1: f32,
               f3,
               f2: f32
          }
          res C
        ",
        expect![[r#"
            Root @0..143
                 Resource @11..21
                      RES @11..14
                      FieldType @15..21
                           IDENT @15..16
                           COLON @16..17
                           Literal @18..21
                                IDENT @18..21
                 Resource @32..118
                      RES @32..35
                      FieldType @36..118
                           IDENT @36..37
                           COLON @37..38
                           StructType @39..118
                                LBRACE @39..40
                                FieldType @56..63
                                     IDENT @56..58
                                     COLON @58..59
                                     Literal @60..63
                                          IDENT @60..63
                                COMMA @63..64
                                FieldType @80..82
                                     IDENT @80..82
                                COMMA @82..83
                                FieldType @99..106
                                     IDENT @99..101
                                     COLON @101..102
                                     Literal @103..106
                                          IDENT @103..106
                                RBRACE @117..118
                 Resource @129..143
                      RES @129..132
                      FieldType @133..143
                           IDENT @133..134
        "#]],
    );
}

#[test]
fn test7() {
    check(
        "
          comp C: f32
        ",
        expect![[r#"
            Root @0..31
                 Component @11..22
                      COMP @11..15
                      FieldType @16..22
                           IDENT @16..17
                           COLON @17..18
                           Literal @19..22
                                IDENT @19..22
        "#]],
    );
}

#[test]
fn test8() {
    check(
        "sys {
          query entity {
               let [Resource] = 123;
               let entity[Component] = 456;
               let a = let b = -123 + 2;
          }
        }",
        expect![[r#"
            Root @0..174
                 System @0..174
                      SYS @0..3
                      Block @4..174
                           LBRACE @4..5
                           Query @16..164
                                QUERY @16..21
                                IDENT @22..28
                                Block @29..164
                                     LBRACE @29..30
                                     Let @46..66
                                          LET @46..49
                                          ResourceAccess @50..60
                                               LBRACKET @50..51
                                               Literal @51..59
                                                    IDENT @51..59
                                               RBRACKET @59..60
                                          EQ @61..62
                                          Literal @63..66
                                               NUMBER @63..66
                                     SEMICOLON @66..67
                                     Let @83..110
                                          LET @83..86
                                          ComponentAccess @87..104
                                               Literal @87..93
                                                    IDENT @87..93
                                               LBRACKET @93..94
                                               Literal @94..103
                                                    IDENT @94..103
                                               RBRACKET @103..104
                                          EQ @105..106
                                          Literal @107..110
                                               NUMBER @107..110
                                     SEMICOLON @110..111
                                     Let @127..151
                                          LET @127..130
                                          Literal @131..132
                                               IDENT @131..132
                                          EQ @133..134
                                          Let @135..151
                                               LET @135..138
                                               Literal @139..140
                                                    IDENT @139..140
                                               EQ @141..142
                                               Infix @143..151
                                                    Prefix @143..148
                                                         MINUS @143..144
                                                         Literal @144..147
                                                              NUMBER @144..147
                                                    PLUS @148..149
                                                    Literal @150..151
                                                         NUMBER @150..151
                                     SEMICOLON @151..152
                                     RBRACE @163..164
                           RBRACE @173..174
        "#]],
    );
}

#[test]
fn test9() {
    check(
        "sys {
          query entity {
               if 1 == 2 {2}
               del 3;
               {7}
               if 4 {5} else {6}
               while 7 {8}
               a(x, y, z)
          }
        }",
        expect![[r#"
            Root @0..208
                 System @0..208
                      SYS @0..3
                      Block @4..208
                           LBRACE @4..5
                           Query @16..198
                                QUERY @16..21
                                IDENT @22..28
                                Block @29..198
                                     LBRACE @29..30
                                     If @46..75
                                          IF @46..48
                                          Infix @49..56
                                               Literal @49..50
                                                    NUMBER @49..50
                                               EQEQ @51..53
                                               Literal @54..55
                                                    NUMBER @54..55
                                          Block @56..59
                                               LBRACE @56..57
                                               Literal @57..58
                                                    NUMBER @57..58
                                               RBRACE @58..59
                                     Del @75..80
                                          DEL @75..78
                                          Literal @79..80
                                               NUMBER @79..80
                                     SEMICOLON @80..81
                                     Block @97..100
                                          LBRACE @97..98
                                          Literal @98..99
                                               NUMBER @98..99
                                          RBRACE @99..100
                                     If @116..133
                                          IF @116..118
                                          Literal @119..120
                                               NUMBER @119..120
                                          Block @121..124
                                               LBRACE @121..122
                                               Literal @122..123
                                                    NUMBER @122..123
                                               RBRACE @123..124
                                          ELSE @125..129
                                          Block @130..133
                                               LBRACE @130..131
                                               Literal @131..132
                                                    NUMBER @131..132
                                               RBRACE @132..133
                                     While @149..160
                                          WHILE @149..154
                                          Literal @155..156
                                               NUMBER @155..156
                                          Block @157..160
                                               LBRACE @157..158
                                               Literal @158..159
                                                    NUMBER @158..159
                                               RBRACE @159..160
                                     Call @176..186
                                          Literal @176..177
                                               IDENT @176..177
                                          LPAREN @177..178
                                          Literal @178..179
                                               IDENT @178..179
                                          COMMA @179..180
                                          Literal @181..182
                                               IDENT @181..182
                                          COMMA @182..183
                                          Literal @184..185
                                               IDENT @184..185
                                          RPAREN @185..186
                                     RBRACE @197..198
                           RBRACE @207..208
        "#]],
    );
}

#[test]
fn test10() {
    check(
        "
          use std
          ",
        expect![[r#"
            Root @0..29
                 Import @11..18
                      USE @11..14
                      IDENT @15..18
        "#]],
    );
}

#[test]
fn test11() {
    check(
        "
          sys {
               A {a: 1, b: 2}
          }
        ",
        expect![[r#"
            Root @0..67
                 System @11..58
                      SYS @11..14
                      Block @15..58
                           LBRACE @15..16
                           Struct @32..46
                                Literal @32..33
                                     IDENT @32..33
                                LBRACE @34..35
                                IDENT @35..36
                                COLON @36..37
                                Literal @38..39
                                     NUMBER @38..39
                                COMMA @39..40
                                IDENT @41..42
                                COLON @42..43
                                Literal @44..45
                                     NUMBER @44..45
                                RBRACE @45..46
                           RBRACE @57..58
        "#]],
    );
}

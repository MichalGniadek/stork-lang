use super::Root;
use crate::cst::{Parser, SyntaxNode};
use expect_test::{expect, Expect};
use rowan::ast::AstNode;

fn check(source: &str, expect: Expect) {
    let mut errors = Vec::new();
    let parser = Parser::new(source, 0, &mut errors);
    let green = parser.parse().unwrap();
    let node = SyntaxNode::new_root(green);
    let root = Root::cast(node).unwrap();
    let result = format!("{root:#?}");
    expect.assert_eq(&result);
}

#[test]
fn test1() {
    check(
        "
    sys {
        query {
            entity[Transform].translation.x = 1;
            [Resource]
        }
    }",
        expect![[r#"
            Root @0..114(
                System @5..114(
                    "Option::None",
                    Block @9..114(
                        Query @19..108(
                            Block @25..108(
                                BinaryExpr @39..74(
                                    BinaryExpr @39..71(
                                        BinaryExpr @39..68(
                                            ECSAccess @39..56(
                                                Some(
                                                    Literal @39..45(
                                                        "entity",
                                                    ),
                                                ),
                                                Literal @46..55(
                                                    "Transform",
                                                ),
                                            ),
                                            DOT@56..57 ".",
                                            Literal @57..68(
                                                "translation",
                                            ),
                                        ),
                                        DOT@68..69 ".",
                                        Literal @69..70(
                                            "x",
                                        ),
                                    ),
                                    EQ@71..72 "=",
                                    Literal @73..74(
                                        "1",
                                    ),
                                ),
                                ECSAccess @88..98(
                                    "Option::None",
                                    Literal @89..97(
                                        "Resource",
                                    ),
                                ),
                            ),
                        ),
                    ),
                ),
            )"#]],
    )
}

#[test]
fn test2() {
    check(
        "
    sys sys_name {
        -x
    }",
        expect![[r#"
            Root @0..36(
                System @5..36(
                    "sys_name",
                    Block @18..36(
                        UnaryExpr @28..35(
                            MINUS@28..29 "-",
                            Literal @29..30(
                                "x",
                            ),
                        ),
                    ),
                ),
            )"#]],
    )
}

#[test]
fn test3() {
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
            Root @0..122(
                Resource @9..19(
                    Field @13..19(
                        "A",
                        IdentifierType @16..19(
                            "f32",
                        ),
                    ),
                ),
                Resource @28..103(
                    Field @32..103(
                        "B",
                        StructType @35..103(
                            Field @49..56(
                                "f1",
                                IdentifierType @53..56(
                                    "f32",
                                ),
                            ),
                            Field @70..72(
                                "f3",
                                "Option::None",
                            ),
                            Field @86..93(
                                "f2",
                                IdentifierType @90..93(
                                    "f32",
                                ),
                            ),
                        ),
                    ),
                ),
                Resource @112..122(
                    Field @116..122(
                        "C",
                        "Option::None",
                    ),
                ),
            )"#]],
    )
}

#[test]
fn test4() {
    check(
        "
          comp C: f32
        ",
        expect![[r#"
            Root @0..31(
                Component @11..22(
                    Field @16..22(
                        "C",
                        IdentifierType @19..22(
                            "f32",
                        ),
                    ),
                ),
            )"#]],
    );
}

#[test]
fn test5() {
    check(
        "sys {
          query entity {
               let [Resource] = 123;
               let entity[Component] = 456;
               let a = let b = -123 + 2;
          }
        }",
        expect![[r#"
            Root @0..174(
                System @0..174(
                    "Option::None",
                    Block @4..174(
                        Query @16..164(
                            Block @29..164(
                                Let @46..66(
                                    ECSAccess @50..60(
                                        "Option::None",
                                        Literal @51..59(
                                            "Resource",
                                        ),
                                    ),
                                    Literal @63..66(
                                        "123",
                                    ),
                                ),
                                Let @83..110(
                                    ECSAccess @87..104(
                                        Some(
                                            Literal @87..93(
                                                "entity",
                                            ),
                                        ),
                                        Literal @94..103(
                                            "Component",
                                        ),
                                    ),
                                    Literal @107..110(
                                        "456",
                                    ),
                                ),
                                Let @127..151(
                                    Literal @131..132(
                                        "a",
                                    ),
                                    Let @135..151(
                                        Literal @139..140(
                                            "b",
                                        ),
                                        BinaryExpr @143..151(
                                            UnaryExpr @143..148(
                                                MINUS@143..144 "-",
                                                Literal @144..147(
                                                    "123",
                                                ),
                                            ),
                                            PLUS@148..149 "+",
                                            Literal @150..151(
                                                "2",
                                            ),
                                        ),
                                    ),
                                ),
                            ),
                        ),
                    ),
                ),
            )"#]],
    );
}

#[test]
fn test6() {
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
            Root @0..208(
                System @0..208(
                    "Option::None",
                    Block @4..208(
                        Query @16..198(
                            Block @29..198(
                                If @46..75(
                                    BinaryExpr @49..56(
                                        Literal @49..50(
                                            "1",
                                        ),
                                        EQEQ@51..53 "==",
                                        Literal @54..55(
                                            "2",
                                        ),
                                    ),
                                    Block @56..59(
                                        Literal @57..58(
                                            "2",
                                        ),
                                    ),
                                    "Option::None",
                                ),
                                Del @75..80(
                                    Literal @79..80(
                                        "3",
                                    ),
                                ),
                                Block @97..100(
                                    Literal @98..99(
                                        "7",
                                    ),
                                ),
                                If @116..133(
                                    Literal @119..120(
                                        "4",
                                    ),
                                    Block @121..124(
                                        Literal @122..123(
                                            "5",
                                        ),
                                    ),
                                    Some(
                                        Block @130..133(
                                            Literal @131..132(
                                                "6",
                                            ),
                                        ),
                                    ),
                                ),
                                While @149..160(
                                    Literal @155..156(
                                        "7",
                                    ),
                                    Block @157..160(
                                        Literal @158..159(
                                            "8",
                                        ),
                                    ),
                                ),
                                Call @176..186(
                                    Literal @176..177(
                                        "a",
                                    ),
                                    [
                                        Literal @178..179(
                                            "x",
                                        ),
                                        Literal @181..182(
                                            "y",
                                        ),
                                        Literal @184..185(
                                            "z",
                                        ),
                                    ],
                                ),
                            ),
                        ),
                    ),
                ),
            )"#]],
    );
}

#[test]
fn test7() {
    check(
        "
          use std
          ",
        expect![[r#"
            Root @0..29(
                Import @11..18(
                    "std",
                ),
            )"#]],
    );
}

#[test]
fn test8() {
    check(
        "
          sys {
               A {a: 1+2, b: 2}
          }
        ",
        expect![[r#"
            Root @0..69(
                System @11..60(
                    "Option::None",
                    Block @15..60(
                        Struct @32..48(
                            "A",
                            (
                                "a",
                                BinaryExpr @38..41(
                                    Literal @38..39(
                                        "1",
                                    ),
                                    PLUS@39..40 "+",
                                    Literal @40..41(
                                        "2",
                                    ),
                                ),
                            ),
                            (
                                "b",
                                Literal @46..47(
                                    "2",
                                ),
                            ),
                        ),
                    ),
                ),
            )"#]],
    );
}

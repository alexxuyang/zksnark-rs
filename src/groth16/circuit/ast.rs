use super::super::super::field::z251::Z251;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
struct TokenList<T> {
    tokens: Vec<Token<T>>,
}

impl<T> IntoIterator for TokenList<T> {
    type Item = Token<T>;
    type IntoIter = ::std::vec::IntoIter<Token<T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.tokens.into_iter()
    }
}

impl<T> From<Vec<Token<T>>> for TokenList<T> {
    fn from(tokens: Vec<Token<T>>) -> Self {
        TokenList { tokens }
    }
}

#[derive(Debug, PartialEq)]
enum Expression<T> {
    In(Vec<Expression<T>>),
    Witness(Vec<Expression<T>>),
    Program(Vec<Expression<T>>),
    Assign(Box<Expression<T>>, Box<Expression<T>>),
    Mul(Box<Expression<T>>, Box<Expression<T>>),
    Add(Box<Expression<T>>, Box<Expression<T>>),
    Var(String),
    Literal(T),
}

#[derive(Clone, Debug, PartialEq)]
enum Key {
    In,
    Witness,
    Program,
    Equal,
    Mul,
    Add,
}

#[derive(Clone, Debug, PartialEq)]
enum Token<T> {
    Keyword(Key),
    Var(String),
    Parenthesis(ParenCase),
    Literal(T),
}

#[derive(Clone, Debug, PartialEq)]
enum ParenCase {
    Open,
    Close,
}

fn parse_expression<T>(token_list: TokenList<T>) -> Result<Expression<T>, ()> {
    use self::Key::*;
    use self::Token::*;

    // Assumes that token_iter is stripped of outer parentheses.
    // This can be achieved by first calling next_group()
    let iter = &mut token_list.into_iter();

    match iter.next() {
        Some(Keyword(k)) => match k {
            In => {
                let mut vars = Vec::new();

                for token in iter {
                    if let Var(v) = token {
                        vars.push(Expression::Var(v));
                    } else {
                        return Err(());
                    }
                }

                Ok(Expression::In(vars))
            }
            Witness => {
                let mut vars = Vec::new();

                for token in iter {
                    if let Var(v) = token {
                        vars.push(Expression::Var(v));
                    } else {
                        return Err(());
                    }
                }

                Ok(Expression::Witness(vars))
            }
            Program => {
                let mut gates = Vec::new();

                loop {
                    let group = next_group(iter);
                    if group.tokens.len() == 0 {
                        break;
                    }

                    let exp = parse_expression(group)?;
                    gates.push(exp);
                }

                Ok(Expression::Program(gates))
            }
            Equal => {
                let left = next_group(iter);
                if left.tokens.len() != 1 {
                    return Err(());
                }
                let left = match left.into_iter().next() {
                    Some(Var(v)) => Expression::Var(v),
                    _ => return Err(()),
                };

                let right = parse_expression(next_group(iter))?;

                Ok(Expression::Assign(Box::new(left), Box::new(right)))
            }
            Mul => {
                let left = parse_expression(next_group(iter))?;
                let right = parse_expression(next_group(iter))?;

                Ok(Expression::Mul(Box::new(left), Box::new(right)))
            }
            Add => {
                let left = parse_expression(next_group(iter))?;
                let right = parse_expression(next_group(iter))?;

                Ok(Expression::Add(Box::new(left), Box::new(right)))
            }
        },
        Some(Var(v)) => Ok(Expression::Var(v)),
        Some(Literal(l)) => Ok(Expression::Literal(l)),
        _ => Err(()),
    }
}

fn next_group<I, T>(token_iter: &mut I) -> TokenList<T>
where
    I: Iterator<Item = Token<T>>,
{
    use self::ParenCase::*;
    use self::Token::*;

    let mut depth = 0;

    match token_iter.next() {
        Some(Parenthesis(Open)) => {
            depth += 1;
            token_iter
                .map(|t| {
                    match t {
                        Parenthesis(Open) => depth += 1,
                        Parenthesis(Close) => depth -= 1,
                        _ => (),
                    }
                    (t, depth)
                })
                .take_while(|&(_, d)| d != 0)
                .map(|(t, _)| t)
                .collect::<Vec<_>>()
                .into()
        }
        Some(v @ Var(_)) => vec![v].into(),
        Some(l @ Literal(_)) => vec![l].into(),
        None => vec![].into(),
        _ => panic!("Cannot parse malformed group"),
    }
}

fn try_to_list<T>(code: String) -> Result<TokenList<T>, ParseErr>
where
    T: FromStr,
{
    use self::ParseErr::*;
    use self::TokenParseErr::*;

    let mut current_line = 1;
    let mut tokens: Vec<Token<T>> = Vec::new();

    for line in code.lines() {
        for substr in line.split_whitespace() {
            match parse_token::<T>(substr) {
                Err(TokenErr(e)) => {
                    return Err(SyntaxErr(current_line, e));
                }
                Ok(ref mut t) => tokens.append(t),
            }
        }

        current_line += 1;
    }

    Ok(TokenList { tokens })
}

#[derive(Debug, PartialEq)]
enum ParseErr {
    SyntaxErr(usize, String),
}

#[derive(Debug, PartialEq)]
enum TokenParseErr {
    TokenErr(String),
}

fn parse_token<T>(mut substr: &str) -> Result<Vec<Token<T>>, TokenParseErr>
where
    T: FromStr,
{
    use self::Key::*;
    use self::ParenCase::*;
    use self::Token::*;
    use self::TokenParseErr::*;

    // Possible valid substrings:
    // ({Keyword}
    // {Var}
    // {Var})
    // {Literal}

    let mut tokens: Vec<Token<T>> = Vec::new();

    if substr.starts_with("(") {
        tokens.push(Parenthesis(Open));
        let (_, s) = substr.split_at(1);
        substr = s;
    }

    if substr.len() == 0 {
        return Err(TokenErr("found whitespace after '('".to_string()));
    }

    match substr {
        "in" => tokens.push(Keyword(In)),
        "witness" => tokens.push(Keyword(Witness)),
        "program" => tokens.push(Keyword(Program)),
        "=" => tokens.push(Keyword(Equal)),
        "*" => tokens.push(Keyword(Mul)),
        "+" => tokens.push(Keyword(Add)),
        _ => {
            if substr.contains("(") {
                return Err(TokenErr("unexpected '('".to_string()));
            } else if substr.contains("*") || substr.contains("+") || substr.contains("=") {
                return Err(TokenErr("unexpected operator".to_string()));
            }

            let (start, end) = split_at_char(substr, ')');
            if tokens.len() != 0 && end.len() != 0 {
                return Err(TokenErr("unexpected ')'".to_string()));
            }

            // It is safe to unwrap because substr.len() >= 1
            let first = start.chars().nth(0).unwrap();

            if first.is_numeric() {
                match start.parse::<T>() {
                    Ok(n) => tokens.push(Literal(n)),
                    _ => return Err(TokenErr("could not parse literal".to_string())),
                }
            } else {
                tokens.push(Var(start.to_owned()));
            }

            for c in end.chars() {
                if c != ')' {
                    return Err(TokenErr("expected ')'".to_string()));
                } else {
                    tokens.push(Parenthesis(Close));
                }
            }
        }
    }

    Ok(tokens)
}

fn split_at_char(s: &str, c: char) -> (&str, &str) {
    let first = &s.chars().take_while(|&x| x != c).collect::<String>();
    s.split_at(first.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_at_char_test() {
        let s = "variable";
        assert_eq!(split_at_char(s, ')'), ("variable", ""));
        let s = "variable)";
        assert_eq!(split_at_char(s, ')'), ("variable", ")"));
        let s = "variable))";
        assert_eq!(split_at_char(s, ')'), ("variable", "))"));
        let s = "variable)))";
        assert_eq!(split_at_char(s, ')'), ("variable", ")))"));
    }

    #[test]
    fn parse_token_test() {
        use self::Key::*;
        use self::ParenCase::*;
        use self::Token::*;
        use self::TokenParseErr::*;

        // Valid substring examples
        let substr = "(in";
        assert_eq!(
            parse_token::<Z251>(substr),
            Ok(vec![Parenthesis(Open), Keyword(In)])
        );
        let substr = "(witness";
        assert_eq!(
            parse_token::<Z251>(substr),
            Ok(vec![Parenthesis(Open), Keyword(Witness)])
        );
        let substr = "(program";
        assert_eq!(
            parse_token::<Z251>(substr),
            Ok(vec![Parenthesis(Open), Keyword(Program)])
        );
        let substr = "(=";
        assert_eq!(
            parse_token::<Z251>(substr),
            Ok(vec![Parenthesis(Open), Keyword(Equal)])
        );
        let substr = "(*";
        assert_eq!(
            parse_token::<Z251>(substr),
            Ok(vec![Parenthesis(Open), Keyword(Mul)])
        );
        let substr = "(+";
        assert_eq!(
            parse_token::<Z251>(substr),
            Ok(vec![Parenthesis(Open), Keyword(Add)])
        );
        let substr = "x";
        assert_eq!(parse_token::<Z251>(substr), Ok(vec![Var("x".to_string())]));
        let substr = "y)";
        assert_eq!(
            parse_token::<Z251>(substr),
            Ok(vec![Var("y".to_string()), Parenthesis(Close)])
        );
        let substr = "y))";
        assert_eq!(
            parse_token::<Z251>(substr),
            Ok(vec![
                Var("y".to_string()),
                Parenthesis(Close),
                Parenthesis(Close),
            ])
        );
        let substr = "9";
        assert_eq!(parse_token::<Z251>(substr), Ok(vec![Literal(9.into())]));
        let substr = "9)";
        assert_eq!(
            parse_token::<Z251>(substr),
            Ok(vec![Literal(9.into()), Parenthesis(Close)])
        );

        // Invalid substring examples
        let substr = "(";
        assert_eq!(
            parse_token::<Z251>(substr),
            Err(TokenErr("found whitespace after '('".to_string()))
        );
        let substr = "(vari(able";
        assert_eq!(
            parse_token::<Z251>(substr),
            Err(TokenErr("unexpected '('".to_string()))
        );
        let substr = "vari(able";
        assert_eq!(
            parse_token::<Z251>(substr),
            Err(TokenErr("unexpected '('".to_string()))
        );
        let substr = "(variable)";
        assert_eq!(
            parse_token::<Z251>(substr),
            Err(TokenErr("unexpected ')'".to_string()))
        );
        let substr = "vari=able";
        assert_eq!(
            parse_token::<Z251>(substr),
            Err(TokenErr("unexpected operator".to_string()))
        );
        let substr = "vari*able";
        assert_eq!(
            parse_token::<Z251>(substr),
            Err(TokenErr("unexpected operator".to_string()))
        );
        let substr = "vari+able";
        assert_eq!(
            parse_token::<Z251>(substr),
            Err(TokenErr("unexpected operator".to_string()))
        );
        let substr = "(vari=able";
        assert_eq!(
            parse_token::<Z251>(substr),
            Err(TokenErr("unexpected operator".to_string()))
        );
        let substr = "(vari*able";
        assert_eq!(
            parse_token::<Z251>(substr),
            Err(TokenErr("unexpected operator".to_string()))
        );
        let substr = "(vari+able";
        assert_eq!(
            parse_token::<Z251>(substr),
            Err(TokenErr("unexpected operator".to_string()))
        );
        let substr = "9variable";
        assert_eq!(
            parse_token::<Z251>(substr),
            Err(TokenErr("could not parse literal".to_string()))
        );
        let substr = "variabl)e))";
        assert_eq!(
            parse_token::<Z251>(substr),
            Err(TokenErr("expected ')'".to_string()))
        );
    }

    #[test]
    fn tokenlist_from_string() {
        use self::Key::*;
        use self::ParenCase::*;
        use self::Token::*;

        let code = "(in x y)
                    (witness a b c)

                    (program
                        (= t1
                            (* x a))
                        (= t2
                            (* x (+ t1 b)))
                        (= y
                            (* 1 (+ t2 c))))";

        let expected = TokenList::<Z251> {
            tokens: vec![
                Parenthesis(Open),
                Keyword(In),
                Var("x".to_string()),
                Var("y".to_string()),
                Parenthesis(Close),
                Parenthesis(Open),
                Keyword(Witness),
                Var("a".to_string()),
                Var("b".to_string()),
                Var("c".to_string()),
                Parenthesis(Close),
                Parenthesis(Open),
                Keyword(Program),
                Parenthesis(Open),
                Keyword(Equal),
                Var("t1".to_string()),
                Parenthesis(Open),
                Keyword(Mul),
                Var("x".to_string()),
                Var("a".to_string()),
                Parenthesis(Close),
                Parenthesis(Close),
                Parenthesis(Open),
                Keyword(Equal),
                Var("t2".to_string()),
                Parenthesis(Open),
                Keyword(Mul),
                Var("x".to_string()),
                Parenthesis(Open),
                Keyword(Add),
                Var("t1".to_string()),
                Var("b".to_string()),
                Parenthesis(Close),
                Parenthesis(Close),
                Parenthesis(Close),
                Parenthesis(Open),
                Keyword(Equal),
                Var("y".to_string()),
                Parenthesis(Open),
                Keyword(Mul),
                Literal(1.into()),
                Parenthesis(Open),
                Keyword(Add),
                Var("t2".to_string()),
                Var("c".to_string()),
                Parenthesis(Close),
                Parenthesis(Close),
                Parenthesis(Close),
                Parenthesis(Close),
            ],
        };

        let actual = try_to_list::<Z251>(code.to_string());

        assert_eq!(Ok(expected), actual);
    }

    #[test]
    fn next_group_test() {
        use self::Token::*;

        let s = "(in x y)";
        let t_list = try_to_list::<Z251>(s.to_string()).unwrap();
        let inner_t_list = try_to_list::<Z251>("in x y".to_string()).unwrap();
        assert_eq!(inner_t_list, next_group(&mut t_list.clone().into_iter()));

        let s = "y (* 1 (+ t2 c)))";
        let t_list = try_to_list::<Z251>(s.to_string()).unwrap();
        let inner_t_list = try_to_list::<Z251>("* 1 (+ t2 c)".to_string()).unwrap();
        let mut iter = t_list.clone().into_iter();
        assert_eq!(next_group(iter.by_ref()), vec![Var("y".to_string())].into());
        assert_eq!(next_group(iter.by_ref()), inner_t_list);
    }

    #[test]
    fn parse_expression_test() {
        use self::Expression::*;

        let code = "(in x y)
                    (witness a b c)

                    (program
                        (= t1
                            (* x a))
                        (= t2
                            (* x (+ t1 b)))
                        (= y
                            (* 1 (+ t2 c))))";
        let token_list = try_to_list::<Z251>(code.to_string()).unwrap();
        let iter = &mut token_list.into_iter();

        let actual = parse_expression(next_group(iter)).unwrap();
        let expected: Expression<Z251> = In(vec![Var("x".to_string()), Var("y".to_string())]);
        assert_eq!(actual, expected);

        let actual = parse_expression(next_group(iter)).unwrap();
        let expected: Expression<Z251> = Witness(vec![
            Var("a".to_string()),
            Var("b".to_string()),
            Var("c".to_string()),
        ]);
        assert_eq!(actual, expected);

        let actual = parse_expression(next_group(iter)).unwrap();
        let expected: Expression<Z251> = Program(vec![
            Assign(
                Box::new(Var("t1".to_string())),
                Box::new(Mul(
                    Box::new(Var("x".to_string())),
                    Box::new(Var("a".to_string())),
                )),
            ),
            Assign(
                Box::new(Var("t2".to_string())),
                Box::new(Mul(
                    Box::new(Var("x".to_string())),
                    Box::new(Add(
                        Box::new(Var("t1".to_string())),
                        Box::new(Var("b".to_string())),
                    )),
                )),
            ),
            Assign(
                Box::new(Var("y".to_string())),
                Box::new(Mul(
                    Box::new(Literal(1.into())),
                    Box::new(Add(
                        Box::new(Var("t2".to_string())),
                        Box::new(Var("c".to_string())),
                    )),
                )),
            ),
        ]);
        assert_eq!(actual, expected);
    }
}

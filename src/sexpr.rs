use std::iter::Peekable;
use std::error::Error;

#[derive(Debug, Clone)]
pub enum Value {
    Nil,
    Cons((Box<Value>, Box<Value>)),
    List(Vec<Value>),
    Key(String),
    Str(String),
    Int(i32),
    Float(f64),
}

impl Value {
    pub fn cons<A, B>(item1:A, item2: B) -> Self
    where
        A: Into<Value>,
        B: Into<Value>,
    {
        Value::Cons((Box::new(item1.into()), Box::new(item2.into())))
    }
}

pub fn parse(string: &str, cons_to_list: bool) -> Result<Value, Box<dyn Error>> {
    let tokens = tokenize(string);
    if tokens.len() == 0 {
        return Ok(Value::Nil);
    }
    parse_sexpr(&mut tokens.into_iter().peekable(), cons_to_list)
}

#[derive(Debug)]
pub enum Token {
    Open,
    Close,
    Dot,
    Str(String),
    Symbol(String),
    Key(String),
    Int(i32),
    Float(f64)
}

pub fn tokenize(string: &str) -> Vec<Token> {
    let mut output = Vec::new();
    if string.len() == 0 {
        return output
    }

    let chars = string.chars().collect::<Vec<char>>();
    let mut pointer = 0;

    while pointer < chars.len() {
        let curr = chars[pointer];
        match curr {
            '(' => {
                output.push(Token::Open);
                pointer += 1;
            },
            ')' => {
                output.push(Token::Close);
                pointer += 1;
            },
            '.' => {
                output.push(Token::Dot);
                pointer += 1;
            },
            '"' => {
                let start = pointer;
                let mut end = start + 1;
                while chars[end] != '"' {
                    end += 1
                }
                output.push(Token::Str(string[start + 1..end].to_string()));
                pointer = end + 1;
            },
            ':' => {
                let start = pointer;
                let mut end = start + 1;
                while !chars[end].is_whitespace() && chars[end] != ')' {
                    end += 1;
                }
                output.push(Token::Key(string[start + 1..end].to_string()));
                pointer = end;
            },
            _ if curr.is_whitespace() => pointer += 1,
            _ => {
                let start = pointer;
                let mut end = start + 1;
                while !chars[end].is_whitespace() && chars[end] != ')' {
                    end += 1;
                }
                let value = string[start..end].to_string();
                if let Ok(value) = value.parse::<i32>() {
                    output.push(Token::Int(value));
                } else if let Ok(value) = value.parse::<f64>() {
                    output.push(Token::Float(value));
                } else {
                    output.push(Token::Symbol(value));
                }
                pointer = end;
            }
        }
    }
    output
}

fn parse_list(tokens: &mut Peekable<impl Iterator<Item=Token>>, cons_to_list: bool) -> Result<Value, Box<dyn Error>> {
    if cons_to_list {
        let mut vec = Vec::new();
        loop {
            match tokens.peek() {
                Some(Token::Close) => {
                    tokens.next();
                    break
                },
                None => return Err("Missing closing ')'".into()),
                _ => vec.push(parse_sexpr(tokens, cons_to_list)?)
            }
        }
        return Ok(Value::List(vec));
    }

    if let Some(Token::Close) = tokens.peek() {
        return Ok(Value::cons(Value::Nil, Value::Nil))
    }

    let first = parse_sexpr(tokens, cons_to_list)?;

    match tokens.peek() {
        Some(Token::Dot) => {
            tokens.next();
            let second = parse_sexpr(tokens, cons_to_list)?;
            match tokens.next() {
                Some(Token::Close) => Ok(Value::cons(first, second)),
                Some(_) => Err(format!("Missing closing parenthesis in dotted pair near {:?}", second).into()),
                None => Err(format!("Unexpected end near {:?}", second).into())
            }
        },
        _ => {
            let rest = parse_list_tail(tokens)?;
            Ok(Value::cons(first, rest))
        }

    }
}

fn parse_list_tail(tokens: &mut Peekable<impl Iterator<Item=Token>>) -> Result<Value, Box<dyn Error>> {
    match tokens.peek() {
        Some(Token::Close) => {
            tokens.next();
            Ok(Value::Nil)
        },
        Some(Token::Dot) => {
            tokens.next();
            let last = parse_sexpr(tokens, false)?;
            match tokens.next() {
                Some(Token::Close) => Ok(last),
                Some(_) => Err(format!("Missing closing parenthesis in dotted pair near {:?}", last).into()),
                None => Err(format!("Unexpected end near {:?}", last).into())
            }
        },
        _ => {
            let first = parse_sexpr(tokens, false)?;
            let rest = parse_list_tail(tokens)?;
            Ok(Value::cons(first, rest))
        }
    }
}

fn parse_sexpr(tokens: &mut Peekable<impl Iterator<Item=Token>>, cons_to_list: bool) -> Result<Value, Box<dyn Error>> {
    println!("Munching {:?}", tokens.peek());
    match tokens.next() {
        Some(Token::Open) => parse_list(tokens, cons_to_list),
        Some(Token::Str(val)) => Ok(Value::Str(val.clone())),
        Some(Token::Symbol(val)) => Ok(Value::Str(val.clone())),
        Some(Token::Key(val)) => Ok(Value::Key(val.clone())),
        Some(Token::Int(val)) => Ok(Value::Int(val.clone())),
        Some(Token::Float(val)) => Ok(Value::Float(val.clone())),
        Some(Token::Dot) if cons_to_list => parse_sexpr(tokens, cons_to_list),
        Some(t) => Err(format!("Unexpected token: {:?}", t).into()),
        None => Err("Unexpected end".into()),
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! cons_test {

    }

    cons_test!{
        ("()", cons("Nil", "Nil")),
        ("(a)", cons("a", "Nil")),
        ("(a . b)", cons("a", "b")),
        ("(a b . c)", cons("a", cons("b", "c"))),
        ("(a b c)", cons("a", cons("b", cons("c", "Nil")))),
        ("((a . b) . c)", cons(cons("a", "b"), "c")),
        ("((a . b) c)", cons(cons("a", "b"), cons("c", "Nil"))),
        ("(a . (b . (c)))", cons("a", cons("b", cons("c", "Nil")))),
        ("((a b) . c)", cons(cons("a", cons("b", "Nil")), "c")),
        ("(a (b . c))", cons("a", cons(cons("b", "c"), "Nil"))),
        ("((a b) (c d))", cons(cons("a", cons("b", "Nil")), cons(cons("c", cons("d", "Nil")), "Nil"))),
        ("((a . (b c)) d)", cons(cons("a", cons("b", cons("c", "Nil"))), cons("d", "Nil"))),
        ("(a b c . d)", cons("a", cons("b", cons("c", "d")))),
        ("((a b . c) d)", cons(cons("a", cons("b", "c")), cons("d", "Nil"))),
        ("(a . (b c))", cons("a", cons("b", cons("c", "Nil")))),
        ("(((a . b) . c) . d)", cons(cons(cons("a", "b"), "c"), "d")),
        ("((a (b c)) . d)", cons(cons("a", cons("b", cons("c", "Nil"))), "d"))
    }
}
*/

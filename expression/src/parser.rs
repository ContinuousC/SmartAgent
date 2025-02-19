/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

//use std::iter::{FromIterator,once};

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1, take_while_m_n},
    character::complete::{anychar, char, digit1, space0},
    combinator::{map, recognize, value},
    error::ErrorKind,
    multi::many1,
    sequence::{delimited, preceded, terminated, tuple},
    IResult,
};
use regex::Regex;

use super::error::EvalError;
use super::expr::Expr;
use unit::parser::valid_composite_unit;
use value::Value;

pub fn parse_expr(input: &str) -> Result<Expr, EvalError> {
    match string_expr(input) {
        Ok(("", e)) => Ok(e),
        Ok((r, _)) => {
            Err(EvalError::ParseError(format!("Leftover input: {}", r)))
        }
        Err(err) => Err(EvalError::ParseError(format!("{}", err))),
    }
}

/* String expressions. */

fn string_expr(input: &str) -> IResult<&str, Expr> {
    let (input, mut elems) = many1(alt((
        variable_reference,
        data_reference,
        embedded_expr,
        string_value,
    )))(input)?;
    let first = elems.remove(0);
    Ok((
        input,
        elems.into_iter().fold(first, |expr, elem| {
            Expr::Concat(Box::new(expr), Box::new(elem))
        }),
    ))
}

fn string_value(input: &str) -> IResult<&str, Expr> {
    map(string1("$@\\{}"), |v| {
        Expr::Literal(Value::UnicodeString(v))
    })(input)
}

fn variable_reference(input: &str) -> IResult<&str, Expr> {
    let (input, _) = char('$')(input)?;
    let (input, val) =
        alt((bracketed_variable_name, simple_variable_name))(input)?;
    Ok((input, Expr::Variable(val)))
}

fn data_reference(input: &str) -> IResult<&str, Expr> {
    let (input, _) = char('@')(input)?;
    /*let (input,val) = alt((bracketed_variable_name,
    simple_variable_name))(input)?;*/
    Ok((input, Expr::Data))
}

fn simple_variable_name(input: &str) -> IResult<&str, String> {
    let (input, name) = take_while1(|c| {
        c >= 'A' && c <= 'Z'
            || c >= 'a' && c <= 'z'
            || c >= '0' && c <= '9'
            || c == '.'
            || c == '_'
    })(input)?;
    Ok((input, String::from(name)))
}

fn bracketed_variable_name(input: &str) -> IResult<&str, String> {
    let (input, _) = char('{')(input)?;
    let (input, name) = string1("\\{}")(input)?;
    let (input, _) = char('}')(input)?;
    Ok((input, name))
}

fn embedded_expr(input: &str) -> IResult<&str, Expr> {
    let (input, _) = char('{')(input)?;
    let (input, expr) = alg_expr(input)?;
    let (input, _) = char('}')(input)?;
    Ok((input, expr))
}

/* Algebraic expression. */

/* Operator table generator.
 * Format: { name, type, { parser => constructor, ... },
 *           ...,
 *           name, { term, ... } }
 * Types:  unary, binary, binary_lassoc, binary_rassoc
 */

macro_rules! operator_table {
    ( $name:ident { $first:ident, $($rest:tt),* } ) => {
	fn $name(input: &str) -> IResult<&str, Expr> {
	    $first(input)
	}
	operator_defs!{$first, $($rest),*}
    }
}

macro_rules! operator_defs {
    ( $name:ident, $type:ident, $row:tt, $next:ident, $($rest:tt),* ) => {
	operator_row!($type, $name, $next, $row);
	operator_defs!($next, $($rest),*);
    };
    ( $name:ident, $row:tt ) => {
	operator_row!(terms, $name, $row);
    };
}

macro_rules! operator_row {
    ( binary_lassoc, $name:ident, $next:ident, { $($parser:expr => $constr:path),+ } ) => {
	fn $name(input: &str) -> IResult<&str, Expr> {
	    let (mut input,mut left) = $next(input)?;
	    loop {
		$(if let Ok((input_,right)) = preceded($parser,$next)(input) {
		    left = $constr(Box::new(left),Box::new(right));
		    input = input_;
		} else )* {
		    return Ok((input,left));
		}
	    }
	}
    };
    ( binary_rassoc, $name:ident, $next:ident, { $($parser:expr => $constr:path),+ } ) => {
	fn $name(input: &str) -> IResult<&str, Expr> {
	    let (input,left) = $next(input)?;
	    $(if let Ok((input,right)) = preceded($parser,$name)(input) {
		Ok((input,$constr(Box::new(left),Box::new(right))))
	    } else )* {
		Ok((input,left))
	    }
	}
    };
    ( binary, $name:ident, $next:ident, { $($parser:expr => $constr:path),+ } ) => {
	fn $name(input: &str) -> IResult<&str, Expr> {
	    let (input,left) = $next(input)?;
	    $(if let Ok((input,right)) = preceded($parser,$next)(input) {
		Ok((input,$constr(Box::new(left),Box::new(right))))
	    } else )* {
		Ok((input,left))
	    }
	}
    };
    ( unary, $name:ident, $next:ident, { $($parser:expr => $constr:path),+ } ) => {
	fn $name(input: &str) -> IResult<&str, Expr> {
	    let (input,_) = space0(input)?;
	    $(if let Ok((input,expr)) = preceded($parser,$next)(input) {
		Ok((input,$constr(Box::new(expr))))
	    } else )* {
		$next(input)
	    }
	}
    };
    ( terms, $name:ident, { $($parser:expr),+ } ) => {
	fn $name(input: &str) -> IResult<&str, Expr> {
	    let (input,_) = space0(input)?;
	    $(if let Ok((input,term)) = terminated($parser, space0)(input) {
		Ok((input,term))
	    } else )* {
		Err(nom::Err::Error(nom::error::Error { input, code: ErrorKind::Alt }))
	    }
	}
    };
}

/* Function table generator.
 * Format: name { function tag (arg : type, ...) => Constructor, ...}
 * Argument types: string, expr
 */

macro_rules! function_table {
    ( $name:ident { $( $fun:ident, $tag:expr, $constr:path, ( $( $arg:ident : $type:ident ),* ) ),+ } ) => {
	fn $name(input: &str) -> IResult<&str,Expr> {
	    $(if let Ok((input,expr)) = $fun(input) {
		Ok((input,expr))
	    } else )* {
		Err(nom::Err::Error(nom::error::Error { input, code: ErrorKind::Alt }))
	    }
	}
	$(fn $fun(input: &str) -> IResult<&str,Expr> {
	    let (input,_) = tag($tag)(input)?;
	    let (input,_) = space0(input)?;
	    let (input,_) = char('(')(input)?;
	    function_args!(input $(, $arg, $type)*);
	    let (input,_) = char(')')(input)?;
	    Ok((input,$constr($(function_arg_constr!($arg,$type)),*)))
	})*
    }
}

macro_rules! function_args {
    ($input:ident) => { };
    ($input:ident, $name:ident, $type:ident) => { let ($input,$name) = function_arg!($type)($input)?; };
    ($input:ident, $name:ident, $type:ident, $($rest:ident),* ) => {
	let ($input,$name) = terminated(function_arg!($type),char(','))($input)?;
	function_args!($input, $($rest),*);
    };
}

macro_rules! function_arg {
    (string) => {
        delimited(space0, string_arg, space0)
    };
    (regex) => {
        delimited(space0, regex_arg, space0)
    };
    (unit) => {
        delimited(space0, valid_composite_unit, space0)
    };
    (expr) => {
        alg_expr
    };
}

macro_rules! function_arg_constr {
    ($name:ident, string) => {
        $name
    };
    ($name:ident, regex) => {
        $name
    };
    ($name:ident, unit) => {
        $name
    };
    ($name:ident, expr) => {
        Box::new($name)
    };
}

/* Operators and functions. */

operator_table! { alg_expr {
    alg_expr_or,    binary_lassoc, { tag("||") => Expr::Or },
    alg_expr_and,   binary_lassoc, { tag("&&") => Expr::And },
    alg_expr_not,   unary,         { char('!') => Expr::Not },
    alg_expr_cmp,   binary,        { tag("<=") => Expr::Le, char('<') => Expr::Lt,
                     tag("==") => Expr::Eq, tag("!=") => Expr::Ne,
                     tag(">=") => Expr::Ge, char('>') => Expr::Gt },
    /* bitwise operators? */
    alg_expr_sum,  binary_lassoc,  { char('+') => Expr::Add, char('-') => Expr::Sub },
    alg_expr_prod, binary_lassoc,  { char('*') => Expr::Mul, char('/') => Expr::Div },
    alg_expr_pow,  binary_rassoc,  { char('^') => Expr::Pow, tag("**") => Expr::Pow },
    alg_expr_neg,  unary,          { char('-') => Expr::Neg },
    alg_expr_term, { opt_unit(variable_reference), opt_unit(data_reference),
             opt_unit(function), opt_unit(float_literal),
             opt_unit(integer_literal), opt_unit(brackets),
             bool_literal, string_literal }
}}

function_table! { function {
    convert_fun,     "convert",     Expr::Convert,    (expr:expr , unit:unit ),
    fallback_fun,    "fallback",    Expr::Fallback,   (expr1:expr, expr2:expr),
    format_fun,      "format",      Expr::Format,     (fmt:string, expr:expr ),
    tostring_fun,    "to_string",   Expr::ToString,   (expr:expr),
    regsubst_fun,    "substitute",  Expr::RegSubst,   (expr:expr, regex:regex, subst:string),
    substr_fun,      "substr",      Expr::SubStr,     (e:expr, f:expr, t:expr),
    concat_fun,      "concat",      Expr::Concat,     (expr1:expr, expr2:expr),

    from_utf8_fun,        "from_utf8",        Expr::FromUtf8,       (expr:expr),
    from_utf8_lossy_fun,  "from_utf8_lossy",  Expr::FromUtf8Lossy,  (expr:expr),
    // from_utf16_fun,       "from_utf16",       Expr::FromUtf16,      (expr:expr),
    // from_utf16_lossy_fun, "from_utf16_lossy", Expr::FromUtf16Lossy, (expr:expr),
    to_binary_fun,        "to_binary",        Expr::ToBinary,       (expr:expr),
    parse_int_fun,        "parse_int",        Expr::ParseInt,       (expr:expr),
    parse_float_fun,      "parse_float",      Expr::ParseFloat,     (expr:expr),
    age_from_seconds,     "age_from_seconds", Expr::AgeFromSeconds, (expr:expr),
    enum_value_fun,       "enum_value",       Expr::EnumValue,      (expr:expr),
    unwrap_error_fun,     "unwrap_error",     Expr::UnwrapError,    (expr:expr),

    parse_mac_bin_fun,   "parse_mac_bin",    Expr::ParseMacBin,        (expr:expr),
    parse_ipv4_bin_fun,  "parse_ipv4_bin",   Expr::ParseIpv4Bin,       (expr:expr),
    parse_ipv6_bin_fun,  "parse_ipv6_bin",   Expr::ParseIpv6Bin,       (expr:expr),

    md5_fun,         "md5",         Expr::MD5,        (expr:expr),
    sha1_fun,        "sha1",        Expr::SHA1,       (expr:expr),

    not_empty_fun,   "not_empty",   Expr::NotEmpty,   (expr:expr),

    log_fun,         "log",         Expr::Log,        (base:expr, expr:expr),
    abs_fun,         "abs",         Expr::Abs,        (expr:expr),
    sign_fun,        "sign",        Expr::Sign,       (expr:expr),
    bits_le_fun,     "bits_le",     Expr::BitsLE,     (n:expr,f:expr,l:expr),
    bits_be_fun,     "bits_be",     Expr::BitsBE,     (n:expr,f:expr,l:expr),

    hex_string_fun,  "hex_string",  Expr::HexStr,     (expr:expr),
    unpack_time_fun, "unpack_time", Expr::UnpackTime, (expr:expr)

}}

fn opt_unit<F>(f: F) -> impl Fn(&str) -> IResult<&str, Expr>
where
    F: Fn(&str) -> IResult<&str, Expr>,
{
    move |input| {
        let (input, term) = f(input)?;
        match preceded(space0, valid_composite_unit)(input) {
            Ok((input, unit)) => {
                Ok((input, Expr::Quantity(Box::new(term), unit)))
            }
            Err(nom::Err::Error(nom::error::Error { input, .. })) => {
                Ok((input, term))
            }
            _ => Err(nom::Err::Error(nom::error::Error {
                input,
                code: ErrorKind::Alt,
            })),
        }
    }
}

fn brackets(input: &str) -> IResult<&str, Expr> {
    let (input, _) = char('(')(input)?;
    let (input, res) = alg_expr(input)?;
    let (input, _) = char(')')(input)?;
    Ok((input, res))
}

fn regex_arg(input: &str) -> IResult<&str, Regex> {
    let (input, res) = string_arg(input)?;
    match Regex::new(&res) {
        Ok(regex) => Ok((input, regex)),
        Err(_) => Err(nom::Err::Error(nom::error::Error {
            input,
            code: ErrorKind::Verify,
        })),
    }
}

fn string_arg(input: &str) -> IResult<&str, String> {
    let (input, c) = alt((char('\''), char('"')))(input)?;
    let (input, res) = string0(if c == '"' { "\\\"" } else { "\\'" })(input)?;
    let (input, _) = char(c)(input)?;
    Ok((input, res))
}

fn string_literal(input: &str) -> IResult<&str, Expr> {
    let (input, c) = alt((char('\''), char('"')))(input)?;
    let (input, res) = string0(if c == '"' { "\\\"" } else { "\\'" })(input)?;
    let (input, _) = char(c)(input)?;
    Ok((input, Expr::Literal(Value::UnicodeString(res))))
}

fn float_literal(input: &str) -> IResult<&str, Expr> {
    let (input, digits) = recognize(tuple((digit1, char('.'), digit1)))(input)?;
    Ok((
        input,
        Expr::Literal(Value::Float(str::parse(digits).unwrap())),
    ))
}

fn integer_literal(input: &str) -> IResult<&str, Expr> {
    let (input, digits) = digit1(input)?;
    Ok((
        input,
        Expr::Literal(Value::Integer(str::parse(digits).unwrap())),
    ))
}

fn bool_literal(input: &str) -> IResult<&str, Expr> {
    alt((
        value(Expr::Literal(Value::Boolean(false)), tag("false")),
        value(Expr::Literal(Value::Boolean(true)), tag("true")),
    ))(input)
}

/* String parser. */

fn string0(reserved: &'static str) -> impl Fn(&str) -> IResult<&str, String> {
    move |mut input| {
        let mut val = String::new();
        loop {
            if let Ok((input_, elem)) =
                take_while1::<_, _, (&str, ErrorKind)>(|c| {
                    !reserved.contains(c)
                })(input)
            {
                val.push_str(elem);
                input = input_;
            } else if let Ok((input_, elem)) = string_escape(input) {
                val.push(elem);
                input = input_;
            } else {
                return Ok((input, val));
            }
        }
    }
}

// fn bytes0(reserved: &'static str) -> impl Fn(&str) -> IResult<&str, Vec<u8>> {
//     move |mut input| {
//         let mut val = Vec::new();
//         loop {
//             if let Ok((input_, elem)) =
//                 take_while1::<_, _, (&str, ErrorKind)>(|c| {
//                     !reserved.contains(c)
//                 })(input)
//             {
//                 val.extend(elem.as_bytes());
//                 input = input_;
//             } else if let Ok((input_, elem)) = byte_escape(input) {
//                 val.extend(elem);
//                 input = input_;
//             } else {
//                 return Ok((input, val));
//             }
//         }
//     }
// }

fn string1(reserved: &'static str) -> impl Fn(&str) -> IResult<&str, String> {
    move |mut input| {
        let mut val = None;
        loop {
            if let Ok((input_, elem)) =
                take_while1::<_, _, (&str, ErrorKind)>(|c| {
                    !reserved.contains(c)
                })(input)
            {
                val.get_or_insert_with(String::new).push_str(elem);
                input = input_;
            } else if let Ok((input_, elem)) = string_escape(input) {
                val.get_or_insert_with(String::new).push(elem);
                input = input_;
            } else {
                return match val {
                    Some(val) => Ok((input, val)),
                    None => Err(nom::Err::Error(nom::error::Error {
                        input,
                        code: ErrorKind::Many1,
                    })),
                };
            }
        }
    }
}

fn string_escape(input: &str) -> IResult<&str, char> {
    let (input, _) = char('\\')(input)?;
    let (input, c) = anychar(input)?;

    match c {
        'x' => {
            let (input, x) =
                take_while_m_n(2, 2, |c: char| c.is_digit(16))(input)?;
            Ok((
                input,
                std::char::from_u32(u32::from_str_radix(x, 16).unwrap())
                    .unwrap(),
            ))
        }
        'u' => {
            let (input, x) =
                take_while_m_n(4, 4, |c: char| c.is_digit(16))(input)?;
            Ok((
                input,
                std::char::from_u32(u32::from_str_radix(x, 16).unwrap())
                    .unwrap(),
            ))
        }
        'r' => Ok((input, '\r')),
        'n' => Ok((input, '\n')),
        't' => Ok((input, '\t')),
        _ => Ok((input, c)),
    }
}

// fn byte_escape(input: &str) -> IResult<&str, Vec<u8>> {
//     let (input, _) = char('\\')(input)?;
//     let (input, c) = anychar(input)?;

//     match c {
//         'x' => {
//             let (input, x) =
//                 take_while_m_n(2, 2, |c: char| c.is_digit(16))(input)?;
//             Ok((input, vec![u8::from_str_radix(x, 16).unwrap()]))
//         }
//         'r' => Ok((input, vec![b'\r'])),
//         'n' => Ok((input, vec![b'\n'])),
//         't' => Ok((input, vec![b'\t'])),
//         _ => Ok((input, format!("{}", c).as_bytes().to_vec())),
//     }
// }

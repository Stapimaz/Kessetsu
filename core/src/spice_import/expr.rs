use std::collections::{BTreeMap, BTreeSet};

use super::component::{is_spice_name, split_fields, unique_identifier};
use super::models::normalize_assignments;
use super::types::{ImportUnit, ImportedParameter, LogicalLine, SpiceImportReport, push_error};

pub(crate) fn convert_rkm_to_standard(input: &str) -> Option<String> {
    let input = input.trim();
    let bytes = input.as_bytes();
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let sign_end = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let int_end = i;
    if int_end == sign_end {
        return None;
    }

    let rem = &input[int_end..];
    let (multiplier_str, mult_len) = if rem.len() >= 3 && rem[..3].eq_ignore_ascii_case("meg") {
        ("meg", 3)
    } else if let Some(&c) = bytes.get(int_end) {
        let lower = c.to_ascii_lowercase();
        if lower == b'r' || lower == b'v' {
            ("", 1)
        } else if matches!(lower, b'k' | b'm' | b'u' | b'n' | b'p' | b'f' | b'g' | b't') {
            (&input[int_end..int_end + 1], 1)
        } else {
            return None;
        }
    } else {
        return None;
    };

    let fraction_start = int_end + mult_len;
    i = fraction_start;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let fraction_end = i;
    if fraction_end == fraction_start {
        return None;
    }

    let trailing = &input[fraction_end..];
    if !trailing.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }

    let sign = &input[..sign_end];
    let integer = &input[sign_end..int_end];
    let fraction = &input[fraction_start..fraction_end];
    Some(format!(
        "{sign}{integer}.{fraction}{multiplier_str}{trailing}"
    ))
}

pub(crate) fn canonical_number(input: &str) -> Result<String, String> {
    let input = input.trim();
    let rkm_converted;
    let input = if let Some(standard) = convert_rkm_to_standard(input) {
        rkm_converted = standard;
        rkm_converted.as_str()
    } else {
        input
    };
    let bytes = input.as_bytes();
    let mut index = usize::from(matches!(bytes.first(), Some(b'+') | Some(b'-')));
    let integer = index;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }
    let mut digits = index > integer;
    if index < bytes.len() && bytes[index] == b'.' {
        index += 1;
        let fraction = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        digits |= index > fraction;
    }
    if !digits {
        return Err(format!("'{input}' is not a literal SPICE number"));
    }
    if index < bytes.len() && matches!(bytes[index], b'e' | b'E') {
        index += 1;
        if index < bytes.len() && matches!(bytes[index], b'+' | b'-') {
            index += 1;
        }
        let exponent = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if exponent == index {
            return Err(format!("'{input}' has an invalid exponent"));
        }
    }
    let number: f64 = input[..index]
        .parse()
        .map_err(|_| format!("'{input}' is not a finite SPICE number"))?;
    let suffix = input[index..].to_ascii_lowercase();
    if !suffix
        .chars()
        .all(|character| character.is_ascii_alphabetic())
    {
        return Err(format!(
            "'{input}' is not a literal SPICE number; expressions are not supported"
        ));
    }
    let factor = if suffix.starts_with("meg") {
        1e6
    } else if suffix.starts_with("mil") {
        25.4e-6
    } else {
        match suffix.chars().next() {
            Some('t') => 1e12,
            Some('g') => 1e9,
            Some('k') => 1e3,
            Some('m') => 1e-3,
            Some('u') => 1e-6,
            Some('n') => 1e-9,
            Some('p') => 1e-12,
            Some('f') => 1e-15,
            Some(character) if character.is_ascii_alphabetic() => 1.0,
            None => 1.0,
            _ => return Err(format!("'{input}' has an unsupported SPICE suffix")),
        }
    };
    let value = number * factor;
    if !value.is_finite() || (number != 0.0 && value == 0.0) {
        return Err(format!("'{input}' is outside the supported numeric range"));
    }
    Ok(if value == 0.0 {
        "0".into()
    } else if factor != 1.0 {
        stable_scientific(value)
    } else {
        value.to_string()
    })
}

pub(crate) fn stable_scientific(value: f64) -> String {
    let formatted = format!("{value:.14e}");
    let (mantissa, exponent) = formatted
        .split_once('e')
        .expect("scientific formatting contains an exponent");
    let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
    let exponent: i32 = exponent.parse().expect("formatted exponent is an integer");
    format!("{mantissa}e{exponent}")
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ExprToken {
    Number(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    StarStar,
    LParen,
    RParen,
    Comma,
}

pub(crate) fn tokenize_spice_expr(expr: &str) -> Result<Vec<ExprToken>, String> {
    let expr = expr.trim();
    let expr = expr
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(expr)
        .trim();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '+' {
            tokens.push(ExprToken::Plus);
            i += 1;
        } else if c == '-' {
            tokens.push(ExprToken::Minus);
            i += 1;
        } else if c == '*' {
            if i + 1 < chars.len() && chars[i + 1] == '*' {
                tokens.push(ExprToken::StarStar);
                i += 2;
            } else {
                tokens.push(ExprToken::Star);
                i += 1;
            }
        } else if c == '/' {
            tokens.push(ExprToken::Slash);
            i += 1;
        } else if c == '^' {
            tokens.push(ExprToken::Caret);
            i += 1;
        } else if c == '(' {
            tokens.push(ExprToken::LParen);
            i += 1;
        } else if c == ')' {
            tokens.push(ExprToken::RParen);
            i += 1;
        } else if c == ',' {
            tokens.push(ExprToken::Comma);
            i += 1;
        } else if c.is_ascii_digit()
            || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            while i < chars.len() {
                let ch = chars[i];
                if ch.is_ascii_alphanumeric()
                    || ch == '.'
                    || ((ch == '+' || ch == '-')
                        && i > start
                        && (chars[i - 1] == 'e' || chars[i - 1] == 'E'))
                {
                    i += 1;
                } else {
                    break;
                }
            }
            let token_str: String = chars[start..i].iter().collect();
            let canonical = canonical_number(&token_str)
                .map_err(|e| format!("invalid number '{token_str}': {e}"))?;
            let val: f64 = canonical
                .parse()
                .map_err(|_| format!("cannot parse number '{token_str}'"))?;
            tokens.push(ExprToken::Number(val));
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let ident: String = chars[start..i].iter().collect();
            tokens.push(ExprToken::Ident(ident));
        } else {
            return Err(format!("unexpected character '{c}' in SPICE expression"));
        }
    }
    Ok(tokens)
}

pub(crate) struct ExprParser<'a> {
    pub(crate) tokens: &'a [ExprToken],
    pub(crate) pos: usize,
    pub(crate) env: &'a BTreeMap<String, f64>,
}

impl<'a> ExprParser<'a> {
    fn peek(&self) -> Option<&ExprToken> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<ExprToken> {
        let tok = self.tokens.get(self.pos).cloned();
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn parse_expr(&mut self) -> Result<f64, String> {
        let mut left = self.parse_term()?;
        while let Some(tok) = self.peek() {
            match tok {
                ExprToken::Plus => {
                    self.next();
                    let right = self.parse_term()?;
                    left += right;
                }
                ExprToken::Minus => {
                    self.next();
                    let right = self.parse_term()?;
                    left -= right;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<f64, String> {
        let mut left = self.parse_power()?;
        while let Some(tok) = self.peek() {
            match tok {
                ExprToken::Star => {
                    self.next();
                    let right = self.parse_power()?;
                    left *= right;
                }
                ExprToken::Slash => {
                    self.next();
                    let right = self.parse_power()?;
                    if right == 0.0 {
                        return Err("division by zero in SPICE expression".into());
                    }
                    left /= right;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<f64, String> {
        let base = self.parse_unary()?;
        if let Some(tok) = self.peek()
            && matches!(tok, ExprToken::Caret | ExprToken::StarStar)
        {
            self.next();
            let exp = self.parse_power()?;
            return Ok(base.powf(exp));
        }
        Ok(base)
    }

    fn parse_unary(&mut self) -> Result<f64, String> {
        if let Some(tok) = self.peek() {
            if matches!(tok, ExprToken::Plus) {
                self.next();
                return self.parse_unary();
            } else if matches!(tok, ExprToken::Minus) {
                self.next();
                return Ok(-self.parse_unary()?);
            }
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<f64, String> {
        match self.next() {
            Some(ExprToken::Number(val)) => Ok(val),
            Some(ExprToken::LParen) => {
                let inner = self.parse_expr()?;
                match self.next().as_ref() {
                    Some(ExprToken::RParen) => Ok(inner),
                    _ => Err("unmatched '(' in SPICE expression".into()),
                }
            }
            Some(ExprToken::Ident(ident)) => {
                let ident_lower = ident.to_ascii_lowercase();
                if self.peek() == Some(&ExprToken::LParen) {
                    if ident_lower == "v" || ident_lower == "i" {
                        return Err(
                            "signal/time-dependent expression is not a constant SPICE parameter"
                                .into(),
                        );
                    }
                    self.next();
                    let arg1 = self.parse_expr()?;
                    if self.peek() == Some(&ExprToken::Comma) {
                        self.next();
                        let arg2 = self.parse_expr()?;
                        if self.next().as_ref() != Some(&ExprToken::RParen) {
                            return Err(format!("expected ')' closing call to '{ident}'"));
                        }
                        match ident_lower.as_str() {
                            "min" => Ok(arg1.min(arg2)),
                            "max" => Ok(arg1.max(arg2)),
                            "pow" => Ok(arg1.powf(arg2)),
                            _ => Err(format!(
                                "unknown 2-argument function '{ident}' in SPICE expression"
                            )),
                        }
                    } else {
                        if self.next().as_ref() != Some(&ExprToken::RParen) {
                            return Err(format!("expected ')' closing call to '{ident}'"));
                        }
                        match ident_lower.as_str() {
                            "sqrt" => {
                                if arg1 < 0.0 {
                                    return Err("sqrt of negative number".into());
                                }
                                Ok(arg1.sqrt())
                            }
                            "sin" => Ok(arg1.sin()),
                            "cos" => Ok(arg1.cos()),
                            "tan" => Ok(arg1.tan()),
                            "asin" => Ok(arg1.asin()),
                            "acos" => Ok(arg1.acos()),
                            "atan" => Ok(arg1.atan()),
                            "exp" => Ok(arg1.exp()),
                            "ln" | "log" => {
                                if arg1 <= 0.0 {
                                    return Err("log of non-positive number".into());
                                }
                                Ok(arg1.ln())
                            }
                            "log10" => {
                                if arg1 <= 0.0 {
                                    return Err("log10 of non-positive number".into());
                                }
                                Ok(arg1.log10())
                            }
                            "abs" => Ok(arg1.abs()),
                            _ => Err(format!("unknown function '{ident}' in SPICE expression")),
                        }
                    }
                } else {
                    if ident_lower == "time" {
                        return Err(
                            "signal/time-dependent expression is not a constant SPICE parameter"
                                .into(),
                        );
                    }
                    if ident_lower == "pi" {
                        return Ok(std::f64::consts::PI);
                    }
                    if ident_lower == "e" {
                        return Ok(std::f64::consts::E);
                    }
                    if let Some(&val) = self.env.get(&ident_lower) {
                        return Ok(val);
                    }
                    Err(format!(
                        "unknown parameter or unsupported symbol '{ident}' in SPICE expression"
                    ))
                }
            }
            Some(tok) => Err(format!("unexpected token '{:?}' in SPICE expression", tok)),
            None => Err("unexpected end of SPICE expression".into()),
        }
    }
}

pub fn evaluate_spice_expr(expr: &str, env: &BTreeMap<String, f64>) -> Result<f64, String> {
    let tokens = tokenize_spice_expr(expr)?;
    if tokens.is_empty() {
        return Err("empty SPICE expression".into());
    }
    let mut parser = ExprParser {
        tokens: &tokens,
        pos: 0,
        env,
    };
    let result = parser.parse_expr()?;
    if parser.pos < tokens.len() {
        return Err(format!(
            "trailing tokens in SPICE expression starting at {:?}",
            tokens[parser.pos]
        ));
    }
    if !result.is_finite() {
        return Err("SPICE expression result is not finite".into());
    }
    Ok(result)
}

pub(crate) fn translate_numeric(
    input: &str,
    unit: ImportUnit,
    parameters: &BTreeMap<String, &ImportedParameter>,
    parameter_units: &mut BTreeMap<String, ImportUnit>,
) -> Result<String, String> {
    let (candidate, braced) = input
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .map_or((input, false), |value| (value.trim(), true));
    if is_spice_name(candidate)
        && let Some(parameter) = parameters.get(&candidate.to_ascii_lowercase())
    {
        let key = parameter.original_name.to_ascii_lowercase();
        if let Some(previous) = parameter_units.insert(key.clone(), unit)
            && previous != unit
        {
            parameter_units.insert(key, previous);
            return Err(format!(
                "parameter '{}' is used as both {} and {}",
                parameter.original_name,
                previous.kessetsu_name(),
                unit.kessetsu_name()
            ));
        }
        return Ok(format!("{{{}}}", parameter.mapped_name));
    }
    if braced {
        let env: BTreeMap<String, f64> = parameters
            .iter()
            .map(|(k, p)| (k.clone(), p.numeric_value))
            .collect();
        let val = evaluate_spice_expr(candidate, &env)
            .map_err(|e| format!("SPICE expression '{{{candidate}}}': {e}"))?;
        return Ok(if val == 0.0 {
            "0".into()
        } else {
            stable_scientific(val)
        });
    }
    canonical_number(input)
}

pub(crate) fn collect_parameters(
    lines: &[LogicalLine],
    report: &mut SpiceImportReport,
) -> Vec<ImportedParameter> {
    let mut parameters = Vec::new();
    let mut seen = BTreeMap::<String, (String, usize)>::new();
    let mut used = BTreeSet::new();
    let mut env = BTreeMap::<String, f64>::new();
    for line in lines {
        let trimmed = line.text.trim();
        let first_token = trimmed.split_whitespace().next();
        if first_token.is_none_or(|field| !field.eq_ignore_ascii_case(".param")) {
            continue;
        }
        let rest = trimmed[".param".len()..].replace(',', " ");
        let normalized = normalize_assignments(&rest);
        let fields = split_fields(normalized.trim());
        if fields.is_empty() {
            push_error(
                report,
                "KES-N006",
                ".param requires at least one NAME=literal assignment",
                Some(line.number),
            );
            continue;
        }
        for assignment in &fields {
            let Some((name, value)) = assignment.split_once('=') else {
                push_error(
                    report,
                    "KES-N006",
                    format!(
                        "parameter assignment '{assignment}' must use NAME=literal without spaces around '='"
                    ),
                    Some(line.number),
                );
                continue;
            };
            if name.is_empty() || value.is_empty() || !is_spice_name(name) {
                push_error(
                    report,
                    "KES-N006",
                    format!("invalid SPICE parameter assignment '{assignment}'"),
                    Some(line.number),
                );
                continue;
            }
            let folded = name.to_ascii_lowercase();
            if let Some((previous, previous_line)) = seen.get(&folded) {
                push_error(
                    report,
                    "KES-N006",
                    format!(
                        "parameter '{name}' duplicates '{previous}' from line {previous_line}; SPICE names are case-insensitive"
                    ),
                    Some(line.number),
                );
                continue;
            }
            seen.insert(folded.clone(), (name.into(), line.number));

            let evaluated = match evaluate_spice_expr(value, &env) {
                Ok(val) => val,
                Err(message) => {
                    push_error(
                        report,
                        "KES-N006",
                        format!(
                            "parameter '{name}' must be a literal or constant arithmetic expression: {message}"
                        ),
                        Some(line.number),
                    );
                    continue;
                }
            };
            env.insert(folded, evaluated);
            let normalized_val = if let Ok(exact) = canonical_number(value) {
                exact
            } else if evaluated == 0.0 {
                "0".into()
            } else {
                stable_scientific(evaluated)
            };
            parameters.push(ImportedParameter {
                original_name: name.into(),
                mapped_name: unique_identifier(name, "P", &mut used),
                value: normalized_val,
                numeric_value: evaluated,
                line: line.number,
            });
        }
    }
    parameters
}

pub(crate) fn parse_parameter_assignments(
    assignments: &[String],
    line: usize,
    used: &mut BTreeSet<String>,
    env: &mut BTreeMap<String, f64>,
) -> Result<Vec<ImportedParameter>, (usize, String)> {
    let mut parameters = Vec::new();
    let mut seen = BTreeMap::<String, String>::new();
    for assignment in assignments {
        let Some((name, value)) = assignment.split_once('=') else {
            return Err((
                line,
                format!(
                    "parameter assignment '{assignment}' must use NAME=literal without spaces around '='"
                ),
            ));
        };
        if name.is_empty() || value.is_empty() || !is_spice_name(name) {
            return Err((
                line,
                format!("invalid SPICE parameter assignment '{assignment}'"),
            ));
        }
        let folded = name.to_ascii_lowercase();
        if let Some(previous) = seen.insert(folded.clone(), name.into()) {
            return Err((
                line,
                format!(
                    "parameter '{name}' duplicates '{previous}'; SPICE names are case-insensitive"
                ),
            ));
        }
        let evaluated = evaluate_spice_expr(value, env).map_err(|message| {
            (
                line,
                format!(
                    "parameter '{name}' must be a literal or constant arithmetic expression: {message}"
                ),
            )
        })?;
        env.insert(folded, evaluated);
        let normalized_val = if let Ok(exact) = canonical_number(value) {
            exact
        } else if evaluated == 0.0 {
            "0".into()
        } else {
            stable_scientific(evaluated)
        };
        parameters.push(ImportedParameter {
            original_name: name.into(),
            mapped_name: unique_identifier(name, "P", used),
            value: normalized_val,
            numeric_value: evaluated,
            line,
        });
    }
    Ok(parameters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rkm_conversion() {
        assert_eq!(convert_rkm_to_standard("4k7"), Some("4.7k".into()));
        assert_eq!(convert_rkm_to_standard("2u2"), Some("2.2u".into()));
        assert_eq!(convert_rkm_to_standard("1r5"), Some("1.5".into()));
        assert_eq!(convert_rkm_to_standard("3v3"), Some("3.3".into()));
        assert_eq!(convert_rkm_to_standard("100n"), None);
        assert_eq!(convert_rkm_to_standard("10M"), None);
        assert_eq!(convert_rkm_to_standard("1meg5"), Some("1.5meg".into()));
        assert_eq!(convert_rkm_to_standard("-2k2"), Some("-2.2k".into()));
    }

    #[test]
    fn test_canonical_number() {
        assert_eq!(canonical_number("4k7").unwrap(), "4.7e3");
        assert_eq!(canonical_number("2.2u").unwrap(), "2.2e-6");
        assert_eq!(canonical_number("100").unwrap(), "100");
        assert_eq!(canonical_number("0").unwrap(), "0");
        assert_eq!(canonical_number("1meg").unwrap(), "1e6");
    }

    #[test]
    fn test_evaluate_spice_expr_arithmetic() {
        let env = BTreeMap::new();
        assert_eq!(evaluate_spice_expr("2 + 3 * 4", &env).unwrap(), 14.0);
        assert_eq!(evaluate_spice_expr("(2 + 3) * 4", &env).unwrap(), 20.0);
        assert_eq!(evaluate_spice_expr("2^3", &env).unwrap(), 8.0);
        assert_eq!(evaluate_spice_expr("2**3", &env).unwrap(), 8.0);
        assert_eq!(evaluate_spice_expr("-5 + 10", &env).unwrap(), 5.0);
        assert_eq!(evaluate_spice_expr("+5 + +5", &env).unwrap(), 10.0);
        assert_eq!(evaluate_spice_expr("10 / 2", &env).unwrap(), 5.0);
    }

    #[test]
    fn test_evaluate_spice_expr_functions_and_constants() {
        let mut env = BTreeMap::new();
        env.insert("r_load".into(), 1000.0);
        assert_eq!(
            evaluate_spice_expr("sqrt(16) + r_load", &env).unwrap(),
            1004.0
        );
        assert_eq!(evaluate_spice_expr("min(10, 20)", &env).unwrap(), 10.0);
        assert_eq!(evaluate_spice_expr("max(10, 20)", &env).unwrap(), 20.0);
        assert_eq!(evaluate_spice_expr("pow(2, 4)", &env).unwrap(), 16.0);
        assert_eq!(evaluate_spice_expr("abs(-42)", &env).unwrap(), 42.0);
        assert!((evaluate_spice_expr("pi", &env).unwrap() - std::f64::consts::PI).abs() < 1e-12);
        assert!((evaluate_spice_expr("e", &env).unwrap() - std::f64::consts::E).abs() < 1e-12);
    }

    #[test]
    fn test_evaluate_spice_expr_errors() {
        let env = BTreeMap::new();
        assert!(evaluate_spice_expr("10 / 0", &env).is_err());
        assert!(evaluate_spice_expr("sqrt(-1)", &env).is_err());
        assert!(evaluate_spice_expr("v(1)", &env).is_err());
        assert!(evaluate_spice_expr("time", &env).is_err());
        assert!(evaluate_spice_expr("unknown_param", &env).is_err());
        assert!(evaluate_spice_expr("(1 + 2", &env).is_err());
    }
}

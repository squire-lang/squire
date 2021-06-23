use crate::value::{Numeral, Text};
use super::{Stream, Result, ErrorKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
	Class,
	Method,
	Field,
	ClassField,
	ClassFn,
	Constructor,
	Function,

	Global,
	Local,

	If,
	Else,
	ComeFrom,
	While,
	Return,
	Try,
	Catch,
	Throw,
	Switch,
	Case,
	Assert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParenKind {
	Round,
	Square,
	Curly
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Symbol {
	Endline,
	Comma,
	Colon,
	Dot,
	Equal,

	EqualEqual,
	NotEqual,
	LessThan,
	LessThanOrEqual,
	GreaterThan,
	GreaterThanOrEqual,
	Compare,

	Plus,
	PlusEqual,
	Hyphen,
	HyphenEqual,
	Asterisk,
	AsteriskEqual,
	Solidus,
	SolidusEqual,
	PercentSign,
	PercentSignEqual,

	Exclamation,
	AndAnd,
	OrOr
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Literal {
	Null,
	Boolean(bool),
	Numeral(Numeral),
	Text(Text), // possibly with interpolation
	TextInterpolation(Vec<(Text, Vec<Token>)>, Text),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
	Keyword(Keyword),
	Symbol(Symbol),
	LeftParen(ParenKind),
	RightParen(ParenKind),
	Literal(Literal),
	StringInterpolation(Vec<(String, Vec<Token>)>, String),
	Identifier(String),
	Label(String),
}

#[derive(Debug)]
pub struct Tokenizer<'a, I> {
	stream: &'a mut Stream<'a, I>,
	macros: Vec<std::vec::IntoIter<Token>>
}

impl<'a, I> Tokenizer<'a, I> {
	pub fn new(stream: &'a mut Stream<'a, I>) -> Self {
		Self { stream, macros: Vec::new() }
	}
}

impl<I: Iterator<Item=char>> Tokenizer<'_, I> {
	pub const CLASS: &'static str           = "form";
	pub const METHOD: &'static str          = "change";
	pub const FIELD: &'static str           = "matter";
	pub const CLASS_FIELD: &'static str     = "essence";
	pub const CLASS_FN: &'static str        = "recall";
	pub const CONSTRUCTOR: &'static str     = "imitate";
	pub const FUNCTION: &'static str        = "journey";

	pub const GLOBAL: &'static str          = "renowned";
	pub const LOCAL: &'static str           = "nigh";

	pub const IF: &'static str              = "if";
	pub const ELSE: &'static str            = "alas";
	pub const COME_FROM: &'static str       = "whence";
	pub const WHILE: &'static str           = "whilst";
	pub const RETURN: &'static str          = "reward";
	pub const TRY: &'static str             = "attempt";
	pub const CATCH: &'static str           = "retreat"; // todo: deprecate
	pub const THROW: &'static str           = "catapult";
	pub const SWITCH: &'static str          = "fork";
	pub const CASE: &'static str            = "path";
	pub const ASSERT: &'static str          = "challenge";

	pub const TRUE: &'static str            = "yay";
	pub const FALSE: &'static str           = "nay";
	pub const NULL: &'static str            = "ni";

	pub fn next_keyword(&mut self) -> Option<Keyword> {
		macro_rules! keyword {
			($string:ident, $name:ident) => {
				if self.stream.take_identifier(Self::$string) {
					return Some(Keyword::$name)
				}
			};
		}

		keyword!(CLASS, Class);
		keyword!(METHOD, Method);
		keyword!(FIELD, Field);
		keyword!(CLASS_FIELD, ClassField);
		keyword!(CLASS_FN, ClassFn);
		keyword!(CONSTRUCTOR, Constructor);
		keyword!(FUNCTION, Function);

		keyword!(GLOBAL, Global);
		keyword!(LOCAL, Local);

		keyword!(IF, If);
		keyword!(ELSE, Else);
		keyword!(COME_FROM, ComeFrom);
		keyword!(WHILE, While);
		keyword!(RETURN, Return);
		keyword!(TRY, Try);
		keyword!(CATCH, Catch);
		keyword!(THROW, Throw);
		keyword!(SWITCH, Switch);
		keyword!(CASE, Case);
		keyword!(ASSERT, Assert);

		None
	}

	fn parse_arabic_numeral(&mut self) -> Option<Result<Numeral>> {
		use crate::value::numeral::NumeralParseError;

		let parsed = self.stream.take_while(|chr| chr.is_ascii_digit() || chr == '_')?;

		match self.stream.peek() {
			Some(chr) if chr.is_alphanumeric() => Some(Err(self.stream.error(NumeralParseError::BadTrailingChar(chr)))),
			_ => Some(Ok(parsed.parse().unwrap()))
		}
	}

	fn parse_roman_numeral(&mut self) -> Option<Result<Numeral>> {
		let parsed = self.stream.take_while(|chr| Numeral::is_roman_numeral(chr) || chr == '_')?;

		// if it's an identifier, then don't raise an error.
		if self.stream.peek().map_or(false, char::is_alphanumeric) {
			self.stream.put_back(parsed.chars());
			return None;
		}

		Some(parsed.parse().map_err(|err| self.stream.error(err)))
	}

	fn parse_numeral(&mut self) -> Option<Result<Token>> {
		self.parse_roman_numeral()
			.or_else(|| self.parse_arabic_numeral())
			.map(|val| val.map(Literal::Numeral).map(Token::Literal))
	}

	fn parse_fraktur(&mut self) -> Result<Text> {
		use crate::value::text::is_fraktur;

		let fraktur = self.stream.take_while(|chr| is_fraktur(chr) || chr.is_whitespace()).unwrap();

		if self.stream.peek().map_or(false, |chr| chr.is_alphanumeric()) {
			Err(self.stream.error(ErrorKind::BadFrakturSuffix))
		} else {
			Ok(Text::new_fraktur(fraktur))
		}
	}

	fn parse_quoted(&mut self) -> Result<Token> {
		let mut text = String::new();

		let quote = self.stream.next().unwrap();
		debug_assert!(quote == '\'' || quote == '\"');

		let mut interpolations = vec![];

		while let Some(chr) = self.stream.next() {
			if chr == quote {
				break;
			} else if chr != '\\' {
				text.push(chr);
				continue;
			} else if quote == '\'' {
				// for single quoting, we only have basic escapes
				match self.stream.next().ok_or_else(|| self.stream.error(ErrorKind::UnterminatedEscapeSequence))? {
					chr @ ('\\' | '\'') => text.push(chr),
					other => { text.push('\\'); text.push(other); }
				}

				continue;
			}

			macro_rules! next_hex_char {
				() => {
					match self.stream.next().map(|chr| (chr, chr.to_digit(16))) {
						Some((_, Some(digit))) => Ok(digit),
						Some((bad, None)) => Err(self.stream.error(ErrorKind::InvalidHexDigit(bad))),
						None => Err(self.stream.error(ErrorKind::UnterminatedEscapeSequence))
					};
				}
			}

			match self.stream.next().ok_or_else(|| self.stream.error(ErrorKind::UnterminatedEscapeSequence))? {
				chr @ ('\\' | '\"' | '\'') => text.push(chr),
				'\r' if self.stream.next() == Some('\n') => continue, // ignore `\` at the end of lines
				'\n' => continue, // ignore `\` at the end of lines
				'n' => text.push('\n'),
				't' => text.push('\t'),
				'r' => text.push('\r'),
				'f' => text.push('\t'),
				'0' => text.push('\0'),
				'x' => {
					let upper = next_hex_char!()? * 0x10;
					let lower = next_hex_char!()? * 0x00;
					let escape = upper | lower;
					text.push(char::from_u32(escape).unwrap());
				},
				'u' => {
					let uppermost = next_hex_char!()? * 0x30;
					let upper     = next_hex_char!()? * 0x20;
					let lower     = next_hex_char!()? * 0x10;
					let lowermost = next_hex_char!()? * 0x00;
					let escape = uppermost | upper | lower | lowermost;

					text.push(char::from_u32(escape).ok_or_else(|| self.stream.error(ErrorKind::InvalidHexEscape(escape)))?);
				},
				'(' => {
					let mut inner = vec![];
					let mut nesting = 1;

					loop {
						let token = self.next()
							.unwrap_or_else(|| Err(self.stream.error(ErrorKind::UnterminatedEscapeSequence)))?;

						if token == Token::LeftParen(ParenKind::Round) {
							nesting += 1;
						} else if token == Token::RightParen(ParenKind::Round) {
							nesting -= 1;
							if nesting == 0 {
								break;
							}
						}

						inner.push(token);
					}

					interpolations.push((Text::new(text), inner));
					text = String::new();
				},
				other => return Err(self.stream.error(ErrorKind::UnknownEscapeCharacter(other)))
			}
		}

		if interpolations.is_empty() {
			Ok(Token::Literal(Literal::Text(Text::new(text))))
		} else {
			Ok(Token::Literal(Literal::TextInterpolation(interpolations, Text::new(text))))
		}
	}

	fn parse_text(&mut self) -> Option<Result<Token>> {
		let peeked = self.stream.peek()?;

		if crate::value::text::is_fraktur(peeked) {
			Some(self.parse_fraktur().map(Literal::Text).map(Token::Literal))
		} else if peeked == '\'' || peeked == '\"' {
			Some(self.parse_quoted())
		} else {
			None
		}
	}

	pub fn next_literal(&mut self) -> Option<Result<Token>> {
		if let Some(numeral) = self.parse_numeral() {
			Some(numeral)
		} else if let Some(text) = self.parse_text() {
			Some(text)
		} else if self.stream.take_identifier(Self::TRUE) {
			Some(Ok(Token::Literal(Literal::Boolean(true))))
		} else if self.stream.take_identifier(Self::FALSE) {
			Some(Ok(Token::Literal(Literal::Boolean(false))))
		} else if self.stream.take_identifier(Self::NULL) {
			Some(Ok(Token::Literal(Literal::Null)))
		} else {
			None
		}
	}

	fn next_from_stream(&mut self) -> Option<Result<Token>> {
		macro_rules! if_equals {
			($if_eql:ident, $if_not:ident) => {
				if self.stream.take_prefix("=") {
					Token::Symbol(Symbol::$if_eql)
				} else {
					Token::Symbol(Symbol::$if_not)
				}
			};
		}

		while self.stream.strip_whitespace() || self.stream.strip_comment() {
			// do nothing
		}

		if let Some(kw) = self.next_keyword() {
			return Some(Ok(Token::Keyword(kw)));
		} else if let Some(literal) = self.next_literal() {
			return Some(literal);
		}

		Some(Ok(match self.stream.next()? {
			// parens
			'(' => Token::LeftParen(ParenKind::Round),
			'[' => Token::LeftParen(ParenKind::Square),
			'{' => Token::LeftParen(ParenKind::Curly),
			')' => Token::RightParen(ParenKind::Round),
			']' => Token::RightParen(ParenKind::Square),
			'}' => Token::RightParen(ParenKind::Curly),

			// symbols
			';' => Token::Symbol(Symbol::Endline),
			',' => Token::Symbol(Symbol::Comma),
			':' => Token::Symbol(Symbol::Colon),
			'.' => Token::Symbol(Symbol::Dot),
			'=' => if_equals!(EqualEqual, Equal),
			'!' => if_equals!(NotEqual, Exclamation),
			'<' => 
				if self.stream.take_prefix("=>") {
					Token::Symbol(Symbol::Compare)
				} else {
					if_equals!(LessThanOrEqual, LessThan)
				},
			'>' => if_equals!(GreaterThanOrEqual, GreaterThan),
			'+' => if_equals!(PlusEqual, Plus),
			'-' => if_equals!(HyphenEqual, Hyphen),
			'*' => if_equals!(AsteriskEqual, Asterisk),
			'/' => if_equals!(SolidusEqual, Solidus),
			'%' => if_equals!(PercentSignEqual, PercentSign),
			'&' if self.stream.take_prefix("&") => Token::Symbol(Symbol::AndAnd),
			'|' if self.stream.take_prefix("|") => Token::Symbol(Symbol::OrOr),
			// '0'..='9' => self.parse_
			other => todo!("parse '{}'", other)
		}))
	}
}

impl<I: Iterator<Item=char>> Iterator for Tokenizer<'_, I> {
	type Item = Result<Token>;

	fn next(&mut self) -> Option<Self::Item> {
		self.next_from_stream()
	}
}

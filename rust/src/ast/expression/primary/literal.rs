use crate::parse::{Parser, Parsable, Error as ParseError};
use crate::parse::token::{Token, TokenKind, Literal as TokenLiteral};
use crate::compile::{Compiler, Compilable, Target, Error as CompileError};

#[derive(Debug)]
pub struct Literal(TokenLiteral);

impl Parsable for Literal {
	const TYPE_NAME: &'static str = "<literal>";

	fn parse<I: Iterator<Item=char>>(parser: &mut Parser<'_, I>) -> Result<Option<Self>, ParseError> {
		// unimplemented: text interpolation...?

		match parser.guard(TokenKind::Literal)? {
			Some(Token::Literal(literal)) => Ok(Some(Self(literal))),
			Some(_) => unreachable!(),
			None => Ok(None)
		}
	}
}

impl Compilable for Literal {
	fn compile(self, compiler: &mut Compiler, target: Option<Target>) -> Result<(), CompileError> {
		use crate::runtime::Opcode;
		use crate::value::Value;

		if target.is_none() && !matches!(self.0, TokenLiteral::TextInterpolation(_, _)) {
			return Ok(());
		}

		let constant_index = 
			match self.0 {
				TokenLiteral::Ni => compiler.get_constant(Value::Ni),
				TokenLiteral::Boolean(boolean) => compiler.get_constant(Value::Veracity(boolean)),
				TokenLiteral::Numeral(numeral) => compiler.get_constant(Value::Numeral(numeral)),
				TokenLiteral::Text(text) => compiler.get_constant(Value::Text(text)),
				TokenLiteral::TextInterpolation(_, _) => unimplemented!()
			};

		if let Some(target) = target {
			compiler.opcode(Opcode::LoadConstant);
			compiler.constant(constant_index);
			compiler.target(target);
		}

		Ok(())
	}
}
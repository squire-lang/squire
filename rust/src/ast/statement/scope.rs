use crate::ast::Expression;
use crate::parse::{Parser, Parsable, Error as ParseError};
use crate::parse::token::{TokenKind, Keyword, Symbol};
use crate::compile::{Compiler, Compilable, Target, Error as CompileError};
use crate::runtime::Opcode;

#[derive(Debug)]
pub struct Renowned {
	name: String,
	init: Option<Expression>
}

#[derive(Debug)]
pub struct Nigh {
	name: String,
	init: Option<Expression>
}

impl Parsable for Renowned {
	const TYPE_NAME: &'static str = Keyword::Global.repr();

	fn parse<I: Iterator<Item=char>>(parser: &mut Parser<'_, I>) -> Result<Option<Self>, ParseError> {
		if parser.guard(TokenKind::Keyword(Keyword::Global))?.is_none() {
			return Ok(None);
		}

		let name = parser.expect_identifier()?;
		let init =
			if parser.guard(TokenKind::Symbol(Symbol::Equal))?.is_some() {
				Some(Expression::expect_parse(parser)?)
			} else {
				None
			};

		Ok(Some(Self { name, init }))
	}
}

impl Parsable for Nigh {
	const TYPE_NAME: &'static str = Keyword::Local.repr();

	fn parse<I: Iterator<Item=char>>(parser: &mut Parser<'_, I>) -> Result<Option<Self>, ParseError> {
		if parser.guard(TokenKind::Keyword(Keyword::Local))?.is_none() {
			return Ok(None);
		}

		let name = parser.expect_identifier()?;
		let init =
			if parser.guard(TokenKind::Symbol(Symbol::Equal))?.is_some() {
				Some(Expression::expect_parse(parser)?)
			} else {
				None
			};

		Ok(Some(Self { name, init }))
	}
}

impl Compilable for Renowned {
	fn compile(self, compiler: &mut Compiler, target: Option<Target>) -> Result<(), CompileError> {
		let global_target = compiler.define_global(self.name, None)?;

		if let Some(init) = self.init {
			let target = target.unwrap_or(Compiler::SCRATCH_TARGET);
			init.compile(compiler, Some(target))?;

			compiler.opcode(Opcode::StoreGlobal);
			compiler.global(global_target);
			compiler.target(target);
		}

		Ok(())
	}
}

impl Compilable for Nigh {
	fn compile(self, compiler: &mut Compiler, target: Option<Target>) -> Result<(), CompileError> {
		let local_target = compiler.define_local(self.name);

		if let Some(init) = self.init {
			init.compile(compiler, Some(local_target))?;
		}

		if let Some(target) = target {
			compiler.opcode(Opcode::Move);
			compiler.target(local_target);
			compiler.target(target);
		}

		Ok(())
	}
}

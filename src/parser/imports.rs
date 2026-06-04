use super::*;

impl Parser {
    pub(super) fn parse_load_statement(&mut self) -> Result<Statement> {
        self.expect(TokenType::Load)?;
        let (path, is_local) = self.parse_import_path()?;

        let alias = if self.check(TokenType::As) {
            self.advance();
            Some(self.expect_ident()?)
        } else {
            None
        };

        let items = if self.check(TokenType::Colon) {
            self.advance();
            self.expect(TokenType::Lparen)?;
            let mut items = Vec::new();
            while !self.check(TokenType::Rparen) && !self.is_at_end() {
                items.push(self.expect_ident()?);
            }
            self.expect(TokenType::Rparen)?;
            Some(items)
        } else {
            None
        };

        if is_local && alias.is_some() {
            return Err(self.error("Local load statements do not support aliasing"));
        }

        Ok(Statement::Load {
            path,
            alias,
            items,
            is_local,
        })
    }

    fn parse_import_path(&mut self) -> Result<(String, bool)> {
        if self.check(TokenType::Dot) {
            self.advance();
            self.expect(TokenType::Slash)?;
            let mut path = String::from("./");
            path.push_str(&self.parse_local_import_segment()?);
            while self.check(TokenType::Slash) {
                self.advance();
                path.push('/');
                path.push_str(&self.parse_local_import_segment()?);
            }
            return Ok((path, true));
        }

        Ok((self.expect_ident()?, false))
    }

    fn parse_local_import_segment(&mut self) -> Result<String> {
        if self.check(TokenType::Dot) && self.peek_n(1).ttype == TokenType::Dot {
            self.advance();
            self.advance();
            return Ok("..".to_string());
        }
        self.expect_member_name()
    }
}

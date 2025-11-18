#[macro_export]
macro_rules! impl_command {
	(
		$(
			$name:ident | $deserialize:expr => $module:ident
		),*
		$(,)?
	) => {
		$(
			use $module::$name;
		)*

		#[derive(Debug, Clone, PartialEq, Eq, Hash, strum_macros::AsRefStr)]
		pub enum Command {
			$(
				$name($name),
			)*
		}

		impl ::std::str::FromStr for Command {
			type Err = String;

			fn from_str(s: &str) -> Result<Self, Self::Err> {
				let (command, params) = s.split_once(' ').unwrap_or((s, ""));
				let (command, params) = (command.trim(), params.trim());

				$(
					if command.eq_ignore_ascii_case($deserialize) {
						let data = <$name as ::std::str::FromStr>::from_str(params)
							.map_err(|e| format!("failed to parse {} command: {}", $deserialize, e))?;

						return Ok(Command::$name(data));
					}
				)*

				Err(format!("unknown command: {}", command))
			}
		}
	};
}

#[macro_export]
macro_rules! unit_commands {
	[$(($mod:ident, $name:ident)),* $(,)?] => {
		$(
			#[allow(dead_code)]
			mod $mod {
				#[derive(Debug, Clone, PartialEq, Eq, Hash)]
				pub struct $name;

				impl ::std::str::FromStr for $name {
					type Err = ::std::convert::Infallible;

					fn from_str(_: &str) -> Result<Self, Self::Err> {
						Ok(Self)
					}
				}
			}
		)*
	};
}

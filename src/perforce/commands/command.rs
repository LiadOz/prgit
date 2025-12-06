use serde::de::DeserializeOwned;
use crate::perforce::p4::P4;
use crate::perforce::error::P4Error;

pub trait P4CommandBase {
    fn command_name() -> &'static str;
    fn p4(&self) -> &P4;
}

pub trait P4Command: P4CommandBase {
    type Response: DeserializeOwned;
    fn args(&self) -> Vec<&str>;
    fn run(&self) -> Result<Self::Response, P4Error> {
        let mut args = vec![Self::command_name()];
        args.extend(self.args());
        let json = self.p4().run(&args)?;
        let response: Self::Response = serde_json::from_value(json)?;
        Ok(response)
    }
}

macro_rules! make_command {
    (
        $struct_name:ident $(<$($lt:lifetime),+>)?,
        $command_name:expr,
        [$($rfield:ident : $rty:ty),* $(,)?]
        $(, $field:ident : $ty:ty )* $(,)?
    ) => {
        pub struct $struct_name $(<$($lt),+>)? {
            p4: &'p P4,
            $( $rfield: $rty, )*
            $( $field: Option<$ty>, )*
        }

        impl $(<$($lt),+>)? $struct_name $(<$($lt),+>)? {
            pub fn new(p4: &'p P4 $(, $rfield: $rty)*) -> Self {
                Self { p4, $($rfield,)* $($field: None,)* }
            }

            $(
            pub fn $field(mut self, value: $ty) -> Self {
                self.$field = Some(value);
                self
            }
            )*
        }

        impl $(<$($lt),+>)? P4CommandBase for $struct_name $(<$($lt),+>)? {
            fn command_name() -> &'static str { $command_name }
            fn p4(&self) -> &P4 { self.p4 }
        }
    }
}

pub(crate) use make_command;


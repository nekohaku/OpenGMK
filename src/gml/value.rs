use crate::{gml, util};
use std::{
    fmt::{self, Display},
    rc::Rc,
};

#[derive(Debug, Clone)]
pub enum Value {
    Real(f64),
    Str(Rc<str>),
}

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Real(r) => write!(f, "{}", r),
            Self::Str(s) => write!(f, "\"{}\"", s.as_ref()),
        }
    }
}

macro_rules! gml_cmp_impl {
    ($($v: vis $fname: ident aka $op_variant: ident: real: $r_cond: expr, string: $s_cond: expr)*) => {
        $(
            $v fn $fname(self, rhs: Self) -> gml::Result<Self> {
                let freal: fn(f64, f64) -> bool = $r_cond;
                let fstr: fn(&str, &str) -> bool = $s_cond;
                if match (self, rhs) {
                    (Self::Real(a), Self::Real(b)) => freal(a, b),
                    (Self::Str(a), Self::Str(b)) => fstr(a.as_ref(), b.as_ref()),
                    (a, b) => return invalid_op!($op_variant, a, b),
                } {
                    Ok(Self::Real(super::TRUE))
                } else {
                    Ok(Self::Real(super::FALSE))
                }
            }
        )*
    };
}

macro_rules! invalid_op {
    ($op: ident, $value: expr) => {
        Err(gml::Error::InvalidOperandsUnary(gml::compiler::token::Operator::$op, $value))
    };
    ($op: ident, $left: expr, $right: expr) => {
        Err(gml::Error::InvalidOperandsBinary(gml::compiler::token::Operator::$op, $left, $right))
    };
}

impl Value {
    // All the GML comparison operators (which return Value not bool).
    #[rustfmt::skip]
    gml_cmp_impl! {
        pub gml_eq aka Equal:
            real: |r1, r2| (r1 - r2).abs() <= 1e-14,
            string: |s1, s2| s1 == s2

        pub gml_ne aka NotEqual:
            real: |r1, r2| (r1 - r2).abs() > 1e-14,
            string: |s1, s2| s1 != s2

        pub gml_lt aka LessThan:
            real: |r1, r2| r1 < r2,
            string: |s1, s2| s1 < s2

        pub gml_lte aka LessThanOrEqual:
            real: |r1, r2| r1 < r2 || (r1 - r2).abs() <= 1e-14,
            string: |s1, s2| s1 <= s2

        pub gml_gt aka GreaterThan:
            real: |r1, r2| r1 > r2,
            string: |s1, s2| s1 > s2

        pub gml_gte aka GreaterThanOrEqual:
            real: |r1, r2| r1 > r2 || (r1 - r2).abs() <= 1e-14,
            string: |s1, s2| s1 >= s2
    }

    pub fn ty_str(&self) -> &'static str {
        match self {
            Self::Real(_) => "real",
            Self::Str(_) => "string",
        }
    }

    /// GML-like comparison, fails if self and other are different types.
    pub fn almost_equals(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Real(a), Self::Real(b)) => (a - b).abs() <= 1e-14,
            (Self::Str(a), Self::Str(b)) => a.as_ref() == b.as_ref(),
            _ => false,
        }
    }

    /// Rounds the value to an i32. This is done very commonly by the GM8 runner.
    pub fn round(&self) -> i32 {
        match &self {
            Self::Real(f) => util::ieee_round(*f),
            Self::Str(_) => 0,
        }
    }

    /// Formats the value as a number or a string with quotes around it so you can see that it is.
    /// Used in generating error messages.
    fn log_fmt(&self) -> String {
        match self {
            Self::Real(real) => real.to_string(),
            Self::Str(string) => format!("\"{}\"", string),
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Real(f) => *f >= 0.5, // What a confusing line.
            Self::Str(_) => false,
        }
    }

    /// Unary bit complement.
    pub fn complement(self) -> gml::Result<Self> {
        match self {
            Self::Real(val) => Ok(Self::Real(!(util::ieee_round(val) as i32) as f64)),
            _ => invalid_op!(Complement, self),
        }
    }

    /// GML operator 'div' which gives the whole number of times RHS can go into LHS. In other words floor(lhs/rhs)
    pub fn intdiv(self, rhs: Self) -> gml::Result<Self> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => Ok(Self::Real((lhs / rhs).floor())),
            (x, y) => invalid_op!(IntDivide, x, y),
        }
    }

    /// GML && operator
    pub fn bool_and(self, rhs: Self) -> gml::Result<Self> {
        Ok(if self.is_truthy() && rhs.is_truthy() { Self::Real(1.0) } else { Self::Real(0.0) })
    }

    /// GML || operator
    pub fn bool_or(self, rhs: Self) -> gml::Result<Self> {
        Ok(if self.is_truthy() || rhs.is_truthy() { Self::Real(1.0) } else { Self::Real(0.0) })
    }

    /// GML ^^ operator
    pub fn bool_xor(self, rhs: Self) -> gml::Result<Self> {
        Ok(if self.is_truthy() != rhs.is_truthy() { Self::Real(1.0) } else { Self::Real(0.0) })
    }

    pub fn add(self, rhs: Self) -> gml::Result<Self> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => Ok(Self::Real(lhs + rhs)),
            (Self::Str(lhs), Self::Str(rhs)) => Ok(Self::Str({
                let mut string = String::with_capacity(lhs.len() + rhs.len());
                string.push_str(lhs.as_ref());
                string.push_str(rhs.as_ref());
                Rc::from(string)
            })),
            (x, y) => invalid_op!(Add, x, y),
        }
    }

    pub fn add_assign(&mut self, rhs: Self) -> gml::Result<()> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => Ok(*lhs += rhs),
            (Self::Str(lhs), Self::Str(ref rhs)) => {
                // TODO: a
                let mut string = String::with_capacity(lhs.len() + rhs.len());
                string.push_str(lhs.as_ref());
                string.push_str(rhs.as_ref());
                *lhs = string.into();
                Ok(())
            },
            (x, y) => invalid_op!(AssignAdd, x.clone(), y),
        }
    }

    pub fn bitand(self, rhs: Self) -> gml::Result<Self> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => {
                Ok(Self::Real((util::ieee_round(lhs) as i32 & util::ieee_round(rhs) as i32) as _))
            },
            (x, y) => invalid_op!(BitwiseAnd, x, y),
        }
    }

    pub fn bitand_assign(&mut self, rhs: Self) -> gml::Result<()> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => {
                Ok(*lhs = (util::ieee_round(*lhs) as i32 & util::ieee_round(rhs) as i32) as _)
            },
            (x, y) => invalid_op!(AssignBitwiseAnd, x.clone(), y),
        }
    }

    pub fn bitor(self, rhs: Self) -> gml::Result<Self> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => {
                Ok(Self::Real((util::ieee_round(lhs) as i32 | util::ieee_round(rhs) as i32) as _))
            },
            (x, y) => invalid_op!(BitwiseOr, x, y),
        }
    }

    pub fn bitor_assign(&mut self, rhs: Self) -> gml::Result<()> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => {
                Ok(*lhs = (util::ieee_round(*lhs) as i32 | util::ieee_round(rhs) as i32) as _)
            },
            (x, y) => invalid_op!(AssignBitwiseOr, x.clone(), y),
        }
    }

    pub fn bitxor(self, rhs: Self) -> gml::Result<Self> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => {
                Ok(Self::Real((util::ieee_round(lhs) as i32 ^ util::ieee_round(rhs) as i32) as _))
            },
            (x, y) => invalid_op!(BitwiseXor, x, y),
        }
    }

    pub fn bitxor_assign(&mut self, rhs: Self) -> gml::Result<()> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => {
                Ok(*lhs = (util::ieee_round(*lhs) as i32 ^ util::ieee_round(rhs) as i32) as _)
            },
            (x, y) => invalid_op!(AssignBitwiseXor, x.clone(), y),
        }
    }

    pub fn div(self, rhs: Self) -> gml::Result<Self> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => Ok(Self::Real(lhs / rhs)),
            (x, y) => invalid_op!(Divide, x, y),
        }
    }

    pub fn div_assign(&mut self, rhs: Self) -> gml::Result<()> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => Ok(*lhs /= rhs),
            (x, y) => invalid_op!(AssignDivide, x.clone(), y),
        }
    }

    pub fn modulo(self, rhs: Self) -> gml::Result<Self> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => Ok(Self::Real(lhs % rhs)),
            (x, y) => invalid_op!(Modulo, x, y),
        }
    }

    pub fn mul(self, rhs: Self) -> gml::Result<Self> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => Ok(Self::Real(lhs * rhs)),
            (Self::Real(lhs), Self::Str(rhs)) => Ok({
                let repeat = util::ieee_round(lhs) as i32;
                if repeat > 0 { rhs.repeat(repeat as usize).into() } else { "".to_string().into() }
            }),
            (x, y) => invalid_op!(Multiply, x, y),
        }
    }

    pub fn mul_assign(&mut self, rhs: Self) -> gml::Result<()> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => Ok(*lhs *= rhs),
            (x, y) => invalid_op!(AssignMultiply, x.clone(), y),
        }
    }

    pub fn neg(self) -> gml::Result<Self> {
        match self {
            Self::Real(f) => Ok(Self::Real(-f)),
            Self::Str(_) => invalid_op!(Subtract, self),
        }
    }

    pub fn not(self) -> gml::Result<Self> {
        match self {
            Self::Real(_) => Ok(Self::Real((!self.is_truthy()) as i8 as f64)),
            Self::Str(_) => invalid_op!(Not, self),
        }
    }

    pub fn shl(self, rhs: Self) -> gml::Result<Self> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => {
                Ok(Self::Real(((util::ieee_round(lhs) as i32) << util::ieee_round(rhs) as i32) as _))
            },
            (x, y) => invalid_op!(BinaryShiftLeft, x, y),
        }
    }

    pub fn shr(self, rhs: Self) -> gml::Result<Self> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => {
                Ok(Self::Real((util::ieee_round(lhs) as i32 >> util::ieee_round(rhs) as i32) as _))
            },
            (x, y) => invalid_op!(BinaryShiftRight, x, y),
        }
    }

    pub fn sub(self, rhs: Self) -> gml::Result<Self> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => Ok(Self::Real(lhs - rhs)),
            (x, y) => invalid_op!(Subtract, x, y),
        }
    }

    pub fn sub_assign(&mut self, rhs: Self) -> gml::Result<()> {
        match (self, rhs) {
            (Self::Real(lhs), Self::Real(rhs)) => Ok(*lhs -= rhs),
            (x, y) => invalid_op!(AssignSubtract, x.clone(), y),
        }
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Real(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Real(value.into())
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Self::Real(value.into())
    }
}

impl From<usize> for Value {
    fn from(value: usize) -> Self {
        Self::Real(value as f64)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Real(if value { gml::TRUE } else { gml::FALSE })
    }
}

impl From<Rc<str>> for Value {
    fn from(value: Rc<str>) -> Self {
        Self::Str(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Str(value.into())
    }
}

impl From<Value> for i32 {
    // For lazy-converting a value into an i32.
    fn from(value: Value) -> Self {
        match value {
            Value::Real(r) => util::ieee_round(r),
            Value::Str(_) => 0,
        }
    }
}

impl From<Value> for u32 {
    // For lazy-converting a value into a u32.
    fn from(value: Value) -> Self {
        match value {
            Value::Real(r) => util::ieee_round(r) as u32,
            Value::Str(_) => 0,
        }
    }
}

impl From<Value> for f64 {
    // For lazy-converting a value into an f64.
    fn from(value: Value) -> Self {
        match value {
            Value::Real(r) => r,
            Value::Str(_) => 0.0,
        }
    }
}

impl From<Value> for Rc<str> {
    // For lazy-converting a value into an Rc<str>.
    fn from(value: Value) -> Self {
        match value {
            Value::Real(_) => String::new().into(),
            Value::Str(s) => s,
        }
    }
}

impl Default for Value {
    fn default() -> Self {
        Self::Real(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_add() {
        let a = Value::Real(0.1);
        let b = Value::Real(0.2);
        assert!((a.add(b).unwrap()).almost_equals(&Value::Real(0.30000000000000004)));

        let c = Value::Str("Hello, ".to_string().into());
        let d = Value::Str("world!".to_string().into());
        assert!((c.add(d).unwrap()).almost_equals(&Value::Str("Hello, world!".to_string().into())));
    }

    #[test]
    #[should_panic]
    fn op_add_invalid() {
        let a = Value::Real(0.1);
        let b = Value::Str("owo".to_string().into());
        let _ = a.add(b).unwrap();
    }
}

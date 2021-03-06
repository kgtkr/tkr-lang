use crate::stream::Stream;
use std::error;
use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;

#[macro_export]
macro_rules! or {
    ($x:expr) => {
        $x
    };
    ($x:expr, $($xs:tt)+) => {
        $x.or(or!($($xs)+))
    };
}

#[derive(Clone, Debug, PartialEq)]
pub enum ErrorExpect<T> {
    Any,
    Eof,
    Token(T),
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParserError<T> {
    pos: usize,
    unexpected: Option<T>,
    expecting: ErrorExpect<T>,
}

impl<T> ParserError<T> {
    pub fn new(pos: usize, unexpected: Option<T>, expecting: ErrorExpect<T>) -> ParserError<T> {
        ParserError {
            pos,
            unexpected,
            expecting,
        }
    }
}

impl<T: Debug> fmt::Display for ParserError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "unexpected {:?} expecting {:?}",
            self.unexpected, self.expecting
        )
    }
}

impl<T: Debug> error::Error for ParserError<T> {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

pub type ParserResult<O, I> = Result<O, ParserError<I>>;

pub trait Parser {
    type Input;
    type Output;
    fn parse(&self, stream: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input>;
    fn map<T, F: Fn(Self::Output) -> T>(self, f: F) -> Map<T, Self, F>
    where
        Self: Sized,
    {
        Map::new(f, self)
    }

    fn attempt(self) -> Attempt<Self>
    where
        Self: Sized,
    {
        Attempt::new(self)
    }

    fn or<T: Parser<Input = Self::Input, Output = Self::Output>>(self, x: T) -> Or<Self, T>
    where
        Self: Sized,
    {
        Or::new(self, x)
    }

    fn and<T: Parser<Input = Self::Input>>(self, x: T) -> And<Self, T>
    where
        Self: Sized,
    {
        And::new(self, x)
    }

    fn val<T: Clone>(self, x: T) -> With<Self, Val<T, Self::Input>>
    where
        Self: Sized,
    {
        self.with(val(x))
    }

    fn with<T: Parser<Input = Self::Input>>(self, x: T) -> With<Self, T>
    where
        Self: Sized,
    {
        With::new(self, x)
    }

    fn skip<T: Parser<Input = Self::Input>>(self, x: T) -> Skip<Self, T>
    where
        Self: Sized,
    {
        Skip::new(self, x)
    }

    fn optional(self) -> Optional<Self>
    where
        Self: Sized,
    {
        Optional::new(self)
    }

    fn many(self) -> Loop<Self>
    where
        Self: Sized,
    {
        Loop::new(self, None, None)
    }

    fn many1(self) -> Loop<Self>
    where
        Self: Sized,
    {
        Loop::new(self, Some(1), None)
    }

    fn many_n(self, n: usize) -> Loop<Self>
    where
        Self: Sized,
    {
        Loop::new(self, Some(n), Some(n))
    }

    fn msg(self, msg: ErrorExpect<Self::Input>) -> Msg<Self>
    where
        Self: Sized,
        Self::Input: Clone,
    {
        Msg::new(self, msg)
    }

    fn then<F: Fn(Self::Output) -> B, B: Parser<Input = Self::Input>>(
        self,
        f: F,
    ) -> Then<Self, F, B>
    where
        Self: Sized,
    {
        Then::new(self, f)
    }

    fn boxed(self) -> Box<Self>
    where
        Self: Sized,
    {
        Box::new(self)
    }
}

impl<A: Parser> Parser for Box<A> {
    type Input = A::Input;
    type Output = A::Output;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        (**self).parse(st)
    }
}

impl<A: Parser> Parser for &A {
    type Input = A::Input;
    type Output = A::Output;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        (**self).parse(st)
    }
}

impl<A: Parser> Parser for &mut A {
    type Input = A::Input;
    type Output = A::Output;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        (**self).parse(st)
    }
}

pub fn any_one<T: Clone>() -> AnyOne<T> {
    AnyOne::new()
}

pub fn eof<T: Clone>() -> Eof<T> {
    Eof::new()
}

pub fn val<T: Clone, I>(x: T) -> Val<T, I> {
    Val::new(x)
}

pub fn token<T: Clone + PartialEq>(x: T) -> Token<T> {
    Token::new(x)
}

pub fn tokens<T: Clone + PartialEq>(x: Vec<T>) -> Tokens<T> {
    Tokens::new(x)
}

pub fn expect<T: Clone, F: Fn(&T) -> bool>(f: F) -> Expect<T, F> {
    Expect::new(f)
}

pub fn parser_func<F: Fn(&mut Stream<A>) -> ParserResult<B, A>, A, B>(f: F) -> ParserFunc<F, A, B> {
    ParserFunc::new(f)
}

pub fn fail<A: Clone, B>() -> Fail<A, B> {
    Fail::new()
}

#[derive(Clone, Debug)]
pub struct AnyOne<T: Clone>(PhantomData<T>);

impl<T: Clone> AnyOne<T> {
    pub fn new() -> Self {
        AnyOne(PhantomData)
    }
}

impl<T: Clone> Parser for AnyOne<T> {
    type Input = T;
    type Output = T;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        let val = st
            .peak()
            .ok_or(ParserError::new(st.pos(), None, ErrorExpect::Any))?;
        st.next();
        Ok(val)
    }
}

#[derive(Clone, Debug)]
pub struct Attempt<T: Parser>(T);

impl<T: Parser> Attempt<T> {
    pub fn new(x: T) -> Self {
        Attempt(x)
    }
}

impl<T: Parser> Parser for Attempt<T> {
    type Input = T::Input;
    type Output = T::Output;
    fn parse(&self, st: &mut Stream<T::Input>) -> ParserResult<T::Output, T::Input> {
        let pos = st.pos();
        let res = self.0.parse(st);
        if let Err(_) = res {
            st.set_pos(pos);
        }
        res
    }
}

#[derive(Clone, Debug)]
pub struct Map<O, T: Parser, F: Fn(T::Output) -> O>(F, T, PhantomData<O>);

impl<O, T: Parser, F: Fn(T::Output) -> O> Map<O, T, F> {
    pub fn new(f: F, x: T) -> Self {
        Map(f, x, PhantomData)
    }
}

impl<O, T: Parser, F: Fn(T::Output) -> O> Parser for Map<O, T, F> {
    type Input = T::Input;
    type Output = O;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        Ok(self.0(self.1.parse(st)?))
    }
}

#[derive(Clone, Debug)]
pub struct Val<T: Clone, I>(T, PhantomData<I>);

impl<T: Clone, I> Val<T, I> {
    pub fn new(x: T) -> Self {
        Val(x, PhantomData)
    }
}

impl<T: Clone, I> Parser for Val<T, I> {
    type Input = I;
    type Output = T;
    fn parse(&self, _: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        Ok(self.0.clone())
    }
}

#[derive(Clone, Debug)]
pub struct Or<A: Parser, B: Parser<Input = A::Input, Output = A::Output>>(A, B);

impl<A: Parser, B: Parser<Input = A::Input, Output = A::Output>> Or<A, B> {
    pub fn new(a: A, b: B) -> Self {
        Or(a, b)
    }
}

impl<A: Parser, B: Parser<Input = A::Input, Output = A::Output>> Parser for Or<A, B> {
    type Input = A::Input;
    type Output = B::Output;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        let pos = st.pos();
        match self.0.parse(st) {
            Err(e) => {
                if pos == st.pos() {
                    self.1.parse(st)
                } else {
                    Err(e)
                }
            }
            x => x,
        }
    }
}

#[derive(Clone, Debug)]
pub struct And<A: Parser, B: Parser<Input = A::Input>>(A, B);

impl<A: Parser, B: Parser<Input = A::Input>> And<A, B> {
    pub fn new(a: A, b: B) -> Self {
        And(a, b)
    }
}

impl<A: Parser, B: Parser<Input = A::Input>> Parser for And<A, B> {
    type Input = A::Input;
    type Output = (A::Output, B::Output);
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        Ok((self.0.parse(st)?, self.1.parse(st)?))
    }
}

#[derive(Clone, Debug)]
pub struct With<A: Parser, B: Parser<Input = A::Input>>(A, B);

impl<A: Parser, B: Parser<Input = A::Input>> With<A, B> {
    pub fn new(a: A, b: B) -> Self {
        With(a, b)
    }
}

impl<A: Parser, B: Parser<Input = A::Input>> Parser for With<A, B> {
    type Input = A::Input;
    type Output = B::Output;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        self.0.parse(st)?;
        self.1.parse(st)
    }
}

#[derive(Clone, Debug)]
pub struct Skip<A: Parser, B: Parser<Input = A::Input>>(A, B);

impl<A: Parser, B: Parser<Input = A::Input>> Skip<A, B> {
    pub fn new(a: A, b: B) -> Self {
        Skip(a, b)
    }
}

impl<A: Parser, B: Parser<Input = A::Input>> Parser for Skip<A, B> {
    type Input = A::Input;
    type Output = A::Output;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        let res = self.0.parse(st)?;
        self.1.parse(st)?;
        Ok(res)
    }
}

#[derive(Clone, Debug)]
pub struct Optional<A: Parser>(A);

impl<A: Parser> Optional<A> {
    pub fn new(a: A) -> Self {
        Optional(a)
    }
}

impl<A: Parser> Parser for Optional<A> {
    type Input = A::Input;
    type Output = Option<A::Output>;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        let pos = st.pos();
        match self.0.parse(st) {
            Err(e) => {
                if pos == st.pos() {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
            Ok(x) => Ok(Some(x)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Loop<A: Parser>(A, Option<usize>, Option<usize>);

impl<A: Parser> Loop<A> {
    pub fn new(a: A, x: Option<usize>, y: Option<usize>) -> Self {
        Loop(a, x, y)
    }
}

impl<A: Parser> Parser for Loop<A> {
    type Input = A::Input;
    type Output = Vec<A::Output>;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        let mut res = Vec::new();
        for i in 0.. {
            if let Some(max) = self.2 {
                if i >= max {
                    break;
                }
            }

            let pos = st.pos();
            match self.0.parse(st) {
                Ok(x) => res.push(x),
                Err(e) => {
                    if let Some(min) = self.1 {
                        if res.len() < min {
                            return Err(e);
                        }
                    }
                    if st.pos() != pos {
                        return Err(e);
                    }
                    break;
                }
            }
        }

        Ok(res)
    }
}

#[derive(Clone, Debug)]
pub struct Eof<T: Clone>(PhantomData<T>);

impl<T: Clone> Eof<T> {
    pub fn new() -> Self {
        Eof(PhantomData)
    }
}

impl<T: Clone> Parser for Eof<T> {
    type Input = T;
    type Output = ();
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        if let Some(x) = st.peak() {
            Err(ParserError::new(st.pos(), Some(x), ErrorExpect::Eof))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug)]
pub struct Token<T: Clone + PartialEq>(T);

impl<T: Clone + PartialEq> Token<T> {
    pub fn new(x: T) -> Self {
        Token(x)
    }
}

impl<T: Clone + PartialEq> Parser for Token<T> {
    type Input = T;
    type Output = T;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        let res = st.peak().ok_or(ParserError::new(
            st.pos(),
            None,
            ErrorExpect::Token(self.0.clone()),
        ))?;
        if res == self.0 {
            st.next();
            Ok(res)
        } else {
            Err(ParserError::new(
                st.pos(),
                Some(res),
                ErrorExpect::Token(self.0.clone()),
            ))
        }
    }
}

#[derive(Clone, Debug)]
pub struct Tokens<T: Clone + PartialEq>(Vec<T>);

impl<T: Clone + PartialEq> Tokens<T> {
    pub fn new(x: Vec<T>) -> Self {
        Tokens(x)
    }
}

impl<T: Clone + PartialEq> Parser for Tokens<T> {
    type Input = T;
    type Output = Vec<T>;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        let mut res = Vec::new();

        for x in self.0.iter() {
            let y = st.peak().ok_or(ParserError::new(
                st.pos(),
                None,
                ErrorExpect::Token(x.clone()),
            ))?;
            if x.clone() == y {
                st.next();
                res.push(y);
            } else {
                return Err(ParserError::new(
                    st.pos(),
                    Some(y),
                    ErrorExpect::Token(x.clone()),
                ));
            }
        }
        Ok(res)
    }
}

#[derive(Clone, Debug)]
pub struct Expect<T: Clone, F: Fn(&T) -> bool>(F, PhantomData<T>);

impl<T: Clone, F: Fn(&T) -> bool> Expect<T, F> {
    pub fn new(f: F) -> Self {
        Expect(f, PhantomData)
    }
}

impl<T: Clone, F: Fn(&T) -> bool> Parser for Expect<T, F> {
    type Input = T;
    type Output = T;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        let x = st
            .peak()
            .ok_or(ParserError::new(st.pos(), None, ErrorExpect::Unknown))?;

        if self.0(&x) {
            st.next();
            Ok(x)
        } else {
            Err(ParserError::new(st.pos(), Some(x), ErrorExpect::Unknown))
        }
    }
}

#[derive(Clone, Debug)]
pub struct Msg<A: Parser>(A, ErrorExpect<A::Input>);

impl<A: Parser> Msg<A>
where
    A::Input: Clone,
{
    pub fn new(a: A, msg: ErrorExpect<A::Input>) -> Self {
        Msg(a, msg)
    }
}

impl<A: Parser> Parser for Msg<A>
where
    A::Input: Clone,
{
    type Input = A::Input;
    type Output = A::Output;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        self.0.parse(st).map_err(|mut e| {
            e.expecting = self.1.clone();
            e
        })
    }
}

#[derive(Clone, Debug)]
pub struct Then<A: Parser, F: Fn(A::Output) -> B, B: Parser<Input = A::Input>>(
    A,
    F,
    PhantomData<B>,
);

impl<A: Parser, F: Fn(A::Output) -> B, B: Parser<Input = A::Input>> Then<A, F, B> {
    pub fn new(a: A, f: F) -> Self {
        Then(a, f, PhantomData)
    }
}

impl<A: Parser, F: Fn(A::Output) -> B, B: Parser<Input = A::Input>> Parser for Then<A, F, B> {
    type Input = A::Input;
    type Output = B::Output;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        match self.0.parse(st) {
            Ok(x) => self.1(x).parse(st),
            Err(e) => Err(e),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ParserFunc<F: Fn(&mut Stream<A>) -> ParserResult<B, A>, A, B>(F, PhantomData<(A, B)>);

impl<F: Fn(&mut Stream<A>) -> ParserResult<B, A>, A, B> ParserFunc<F, A, B> {
    pub fn new(f: F) -> Self {
        ParserFunc(f, PhantomData)
    }
}

impl<F: Fn(&mut Stream<A>) -> ParserResult<B, A>, A, B> Parser for ParserFunc<F, A, B> {
    type Input = A;
    type Output = B;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        self.0(st)
    }
}

#[derive(Clone, Debug)]
pub struct Fail<A: Clone, B>(PhantomData<(A, B)>);

impl<A: Clone, B> Fail<A, B> {
    pub fn new() -> Self {
        Fail(PhantomData)
    }
}

impl<A: Clone, B> Parser for Fail<A, B> {
    type Input = A;
    type Output = B;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        Err(ParserError::new(
            st.pos(),
            st.peak().map(Some).unwrap_or(None),
            ErrorExpect::Unknown,
        ))
    }
}

#[derive(Clone, Debug)]
pub enum Either<A: Parser, B: Parser<Input = A::Input, Output = A::Output>> {
    Right(A),
    Left(B),
}

impl<A: Parser, B: Parser<Input = A::Input, Output = A::Output>> Parser for Either<A, B> {
    type Input = A::Input;
    type Output = A::Output;
    fn parse(&self, st: &mut Stream<Self::Input>) -> ParserResult<Self::Output, Self::Input> {
        match self {
            Either::Right(x) => x.parse(st),
            Either::Left(x) => x.parse(st),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn helper<A: Parser>(
        analyzer: A,
        cases: Vec<(Vec<A::Input>, ParserResult<A::Output, A::Input>, usize)>,
    ) where
        A::Input: PartialEq + Debug,
        A::Output: PartialEq + Debug,
    {
        for (input, result, pos) in cases {
            let mut st = Stream::new(input);
            assert_eq!(result, analyzer.parse(&mut st));
            assert_eq!(pos, st.pos());
        }
    }

    #[test]
    fn map_test() {
        helper(
            token(1).map(|x| x + 1),
            vec![
                (vec![1], Ok(2), 1),
                (
                    vec![2],
                    Err(ParserError::new(0, Some(2), ErrorExpect::Token(1))),
                    0,
                ),
            ],
        );
    }

    #[test]
    fn attempt_test() {
        helper(
            tokens(vec![1, 2]).attempt(),
            vec![
                (vec![1, 2], Ok(vec![1, 2]), 2),
                (
                    vec![1, 3],
                    Err(ParserError::new(1, Some(3), ErrorExpect::Token(2))),
                    0,
                ),
            ],
        );
    }

    #[test]
    fn any_one_test() {
        helper(
            any_one(),
            vec![
                (vec![1], Ok(1), 1),
                (vec![], Err(ParserError::new(0, None, ErrorExpect::Any)), 0),
            ],
        );
    }

    #[test]
    fn val_test() {
        helper(val(2), vec![(vec![1], Ok(2), 0)]);
    }

    #[test]
    fn or_test() {
        helper(
            token(1).or(token(2)),
            vec![
                (vec![1], Ok(1), 1),
                (vec![2], Ok(2), 1),
                (
                    vec![3],
                    Err(ParserError::new(0, Some(3), ErrorExpect::Token(2))),
                    0,
                ),
            ],
        );

        helper(
            tokens(vec![1, 2]).or(tokens(vec![1, 3])),
            vec![
                (
                    vec![1, 3],
                    Err(ParserError::new(1, Some(3), ErrorExpect::Token(2))),
                    1,
                ),
                (
                    vec![1, 1, 3],
                    Err(ParserError::new(1, Some(1), ErrorExpect::Token(2))),
                    1,
                ),
            ],
        );
    }
}

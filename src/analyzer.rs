use crate::stream::Stream;
use std::marker::PhantomData;

pub trait Analyzer {
    type Input;
    type Output;
    fn analyze(&self, stream: &mut Stream<Self::Input>) -> Option<Self::Output>;
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

    fn or<T: Analyzer<Input = Self::Input, Output = Self::Output>>(self, x: T) -> Or<Self, T>
    where
        Self: Sized,
    {
        Or::new(self, x)
    }

    fn and<T: Analyzer<Input = Self::Input>>(self, x: T) -> And<Self, T>
    where
        Self: Sized,
    {
        And::new(self, x)
    }

    fn with<T: Analyzer<Input = Self::Input>>(self, x: T) -> With<Self, T>
    where
        Self: Sized,
    {
        With::new(self, x)
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
}

pub fn anyOne<T: Clone>() -> AnyOne<T> {
    AnyOne::new()
}

pub fn eof<T: Analyzer>() -> Eof<T> {
    Eof::new()
}

pub fn val<T: Clone, A: Analyzer>(x: T) -> Val<T, A> {
    Val::new(x)
}

pub struct AnyOne<T: Clone>(PhantomData<T>);

impl<T: Clone> AnyOne<T> {
    pub fn new() -> Self {
        AnyOne(PhantomData)
    }
}

impl<T: Clone> Analyzer for AnyOne<T> {
    type Input = T;
    type Output = T;
    fn analyze(&self, st: &mut Stream<Self::Input>) -> Option<Self::Output> {
        let val = st.peak()?;
        st.add_pos(1);
        Some(val)
    }
}

pub struct Attempt<T: Analyzer>(T);

impl<T: Analyzer> Attempt<T> {
    pub fn new(x: T) -> Self {
        Attempt(x)
    }
}

impl<T: Analyzer> Analyzer for Attempt<T> {
    type Input = T::Input;
    type Output = T::Output;
    fn analyze(&self, st: &mut Stream<T::Input>) -> Option<T::Output> {
        let pos = st.pos();
        let res = self.0.analyze(st);
        if let None = res {
            st.set_pos(pos);
        }
        res
    }
}

pub struct Map<O, T: Analyzer, F: Fn(T::Output) -> O>(F, T, PhantomData<O>);

impl<O, T: Analyzer, F: Fn(T::Output) -> O> Map<O, T, F> {
    pub fn new(f: F, x: T) -> Self {
        Map(f, x, PhantomData)
    }
}

impl<O, T: Analyzer, F: Fn(T::Output) -> O> Analyzer for Map<O, T, F> {
    type Input = T::Input;
    type Output = O;
    fn analyze(&self, st: &mut Stream<Self::Input>) -> Option<Self::Output> {
        Some(self.0(self.1.analyze(st)?))
    }
}

pub struct Val<T: Clone, A: Analyzer>(T, PhantomData<A>);

impl<T: Clone, A: Analyzer> Val<T, A> {
    pub fn new(x: T) -> Self {
        Val(x, PhantomData)
    }
}

impl<T: Clone, A: Analyzer> Analyzer for Val<T, A> {
    type Input = A::Input;
    type Output = T;
    fn analyze(&self, _: &mut Stream<Self::Input>) -> Option<Self::Output> {
        Some(self.0.clone())
    }
}

pub struct Or<A: Analyzer, B: Analyzer<Input = A::Input, Output = A::Output>>(A, B);

impl<A: Analyzer, B: Analyzer<Input = A::Input, Output = A::Output>> Or<A, B> {
    pub fn new(a: A, b: B) -> Self {
        Or(a, b)
    }
}

impl<A: Analyzer, B: Analyzer<Input = A::Input, Output = A::Output>> Analyzer for Or<A, B> {
    type Input = A::Input;
    type Output = B::Output;
    fn analyze(&self, st: &mut Stream<Self::Input>) -> Option<Self::Output> {
        match self.0.analyze(st) {
            None => self.1.analyze(st),
            x => x,
        }
    }
}

pub struct And<A: Analyzer, B: Analyzer<Input = A::Input>>(A, B);

impl<A: Analyzer, B: Analyzer<Input = A::Input>> And<A, B> {
    pub fn new(a: A, b: B) -> Self {
        And(a, b)
    }
}

impl<A: Analyzer, B: Analyzer<Input = A::Input>> Analyzer for And<A, B> {
    type Input = A::Input;
    type Output = (A::Output, B::Output);
    fn analyze(&self, st: &mut Stream<Self::Input>) -> Option<Self::Output> {
        Some((self.0.analyze(st)?, self.1.analyze(st)?))
    }
}

pub struct With<A: Analyzer, B: Analyzer<Input = A::Input>>(A, B);

impl<A: Analyzer, B: Analyzer<Input = A::Input>> With<A, B> {
    pub fn new(a: A, b: B) -> Self {
        With(a, b)
    }
}

impl<A: Analyzer, B: Analyzer<Input = A::Input>> Analyzer for With<A, B> {
    type Input = A::Input;
    type Output = B::Output;
    fn analyze(&self, st: &mut Stream<Self::Input>) -> Option<Self::Output> {
        self.0.analyze(st)?;
        self.1.analyze(st)
    }
}

pub struct Optional<A: Analyzer>(A);

impl<A: Analyzer> Optional<A> {
    pub fn new(a: A) -> Self {
        Optional(a)
    }
}

impl<A: Analyzer> Analyzer for Optional<A> {
    type Input = A::Input;
    type Output = Option<A::Output>;
    fn analyze(&self, st: &mut Stream<Self::Input>) -> Option<Self::Output> {
        Some(self.0.analyze(st))
    }
}

pub struct Loop<A: Analyzer>(A, Option<usize>, Option<usize>);

impl<A: Analyzer> Loop<A> {
    pub fn new(a: A, x: Option<usize>, y: Option<usize>) -> Self {
        Loop(a, x, y)
    }
}

impl<A: Analyzer> Analyzer for Loop<A> {
    type Input = A::Input;
    type Output = Vec<A::Output>;
    fn analyze(&self, st: &mut Stream<Self::Input>) -> Option<Self::Output> {
        let mut res = Vec::new();
        for i in 0.. {
            if let Some(max) = self.2 {
                if i >= max {
                    break;
                }
            }

            match self.0.analyze(st) {
                Some(x) => res.push(x),
                None => break,
            }
        }

        if let Some(min) = self.1 {
            if res.len() < min {
                return None;
            }
        }

        Some(res)
    }
}

pub struct Eof<A: Analyzer>(PhantomData<A>);

impl<A: Analyzer> Eof<A> {
    pub fn new() -> Self {
        Eof(PhantomData)
    }
}

impl<A: Analyzer> Analyzer for Eof<A> {
    type Input = A::Input;
    type Output = ();
    fn analyze(&self, st: &mut Stream<Self::Input>) -> Option<Self::Output> {
        if st.eof() {
            Some(())
        } else {
            None
        }
    }
}

//! class! 宏展开验证

use tabookit::class;

// 无泛型
class! {
    #[derive(Debug, Clone)]
    pub struct Foo {
        pub x: i32,
        y: String,
    }

    pub fn new(x: i32) -> Self {
        Self { x, y: String::new() }
    }

    fn y(&self) -> &str {
        &self.y
    }
}

// 单类型参数 + bound
class! {
    #[derive(Debug)]
    pub struct Wrapper<T: Clone> {
        inner: T,
    }

    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    pub fn get(&self) -> &T {
        &self.inner
    }
}

// 多类型参数 + 多 bound
class! {
    struct Multi<T: Clone + Send, U: Default> {
        a: T,
        b: U,
    }

    fn new(a: T) -> Self {
        Self { a, b: U::default() }
    }
}

// lifetime 参数
class! {
    struct Borrowed<'a, T: 'a> {
        data: &'a T,
    }

    fn data(&self) -> &T {
        self.data
    }
}

// bound 中含嵌套泛型: Iterator<Item = i32>
class! {
    struct Iter<T: Iterator<Item = i32>> {
        iter: T,
    }

    fn into_inner(self) -> T {
        self.iter
    }
}

#[test]
fn test_no_generics() {
    let foo = Foo::new(42);
    assert_eq!(foo.x, 42);
    assert_eq!(foo.y(), "");
    let _ = foo.clone();
}

#[test]
fn test_single_generic() {
    let w = Wrapper::new(42_i32);
    assert_eq!(*w.get(), 42);
}

#[test]
fn test_multi_generic() {
    let m = Multi::<i32, String>::new(1);
    assert_eq!(m.a, 1);
    assert_eq!(m.b, "");
}

#[test]
fn test_lifetime() {
    let val = 42;
    let b = Borrowed { data: &val };
    assert_eq!(*b.data(), 42);
}

#[test]
fn test_nested_bound() {
    let v = vec![1, 2, 3];
    let it = Iter {
        iter: v.into_iter(),
    };
    let collected: Vec<_> = it.into_inner().collect();
    assert_eq!(collected, vec![1, 2, 3]);
}

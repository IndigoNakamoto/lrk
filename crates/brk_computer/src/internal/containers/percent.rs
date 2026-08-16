use brk_traversable::Traversable;

#[derive(Clone, Traversable)]
pub struct Percent<A, B = A, C = B> {
    pub ppm: A,
    pub ratio: B,
    pub percent: C,
}

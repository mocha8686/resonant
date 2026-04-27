use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use num_traits::Num;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Vector2<T = f32> {
    pub x: T,
    pub y: T,
}

impl Vector2 {
    pub const ZERO: Self = Self::new(0.0, 0.0);
    pub const RIGHT: Self = Self::new(1.0, 0.0);
    pub const UP: Self = Self::new(0.0, 1.0);
}

impl<T> Vector2<T> {
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T> Vector2<T>
where
    T: Copy + Add<Output = T> + Mul<Output = T>,
{
    pub fn square_magnitude(self) -> T {
        self.x * self.x + self.y * self.y
    }

    pub fn dot(self, other: Self) -> T {
        self.x * other.x + self.y * other.y
    }
}

impl Vector2<f32> {
    #[must_use]
    pub fn magnitude(self) -> f32 {
        self.square_magnitude().sqrt()
    }

    #[must_use]
    pub fn normalized(self) -> Self {
        self / self.magnitude()
    }
}

impl Vector2<f64> {
    #[must_use]
    pub fn magnitude(self) -> f64 {
        self.square_magnitude().sqrt()
    }

    #[must_use]
    pub fn normalized(self) -> Self {
        self / self.magnitude()
    }
}

impl<T: Copy + Default> Default for Vector2<T> {
    fn default() -> Self {
        Self {
            x: T::default(),
            y: T::default(),
        }
    }
}

impl<T> From<iced::Vector<T>> for Vector2<T> {
    fn from(value: iced::Vector<T>) -> Self {
        Self::new(value.x, value.y)
    }
}

impl<T: Copy> From<&iced::Vector<T>> for Vector2<T> {
    fn from(value: &iced::Vector<T>) -> Self {
        Self::new(value.x, value.y)
    }
}

impl<T> From<Vector2<T>> for iced::Vector<T> {
    fn from(value: Vector2<T>) -> Self {
        Self::new(value.x, value.y)
    }
}

impl<T: Copy> From<&Vector2<T>> for iced::Vector<T> {
    fn from(value: &Vector2<T>) -> Self {
        Self::new(value.x, value.y)
    }
}

impl<T> From<iced::Point<T>> for Vector2<T> {
    fn from(value: iced::Point<T>) -> Self {
        Self::new(value.x, value.y)
    }
}

impl<T: Copy> From<&iced::Point<T>> for Vector2<T> {
    fn from(value: &iced::Point<T>) -> Self {
        Self::new(value.x, value.y)
    }
}

impl<T: Num> From<Vector2<T>> for iced::Point<T> {
    fn from(value: Vector2<T>) -> Self {
        Self::new(value.x, value.y)
    }
}

impl<T: Copy + Num> From<&Vector2<T>> for iced::Point<T> {
    fn from(value: &Vector2<T>) -> Self {
        Self::new(value.x, value.y)
    }
}

impl<T> From<[T; 2]> for Vector2<T> {
    fn from([x, y]: [T; 2]) -> Self {
        Self::new(x, y)
    }
}

impl<T> From<(T, T)> for Vector2<T> {
    fn from((x, y): (T, T)) -> Self {
        Self::new(x, y)
    }
}

impl<T: Neg<Output = T>> Neg for Vector2<T> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

impl<T: Add<Output = T>> Add for Vector2<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<T: AddAssign> AddAssign for Vector2<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<T: Sub<Output = T>> Sub for Vector2<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<T: SubAssign> SubAssign for Vector2<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<T: Mul<Output = T> + Copy> Mul<T> for Vector2<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl<T: MulAssign + Copy> MulAssign<T> for Vector2<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl<T: Div<Output = T> + Copy> Div<T> for Vector2<T> {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl<T: DivAssign + Copy> DivAssign<T> for Vector2<T> {
    fn div_assign(&mut self, rhs: T) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

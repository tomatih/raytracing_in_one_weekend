use std::ops;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Vec3{
    e: [f32; 3],
}

// aliases
pub type Point3 = Vec3;
pub type Color = Vec3;

impl Vec3 {
    // constructors
    pub fn new() -> Self{
        Vec3 { e: [0.0, 0.0, 0.0] }
    }
    pub fn from_values(x:f32, y:f32, z:f32) -> Self{
        Vec3 { e: [x, y, z] }
    }
    // accessors
    pub fn x(self) -> f32 { self.e[0] }
    pub fn y(self) -> f32 { self.e[1] }
    pub fn z(self) -> f32 { self.e[2] }
    // lenght
    fn length_squared(&self) -> f32{
        self.e[0] * self.e[0] + self.e[1] * self.e[1] + self.e[2] * self.e[2]
    }
    pub fn length(&self) -> f32{
        f32::sqrt(self.length_squared())
    }
    // operations
    pub fn dot(a: Vec3, b: Vec3) -> f32{
        a[0]*b[0] + a[1]*b[1] + a[2]*b[2]
    }
    pub fn cross(a: Vec3, b:Vec3) -> Vec3{
        Vec3 { e: [
            a[1]*b[2] - a[2]*b[1],
            a[2]*b[0] - a[0]*b[2],
            a[0]*b[1] - a[1]*b[0],
        ] }
    }
    pub fn unit_vector(v: Vec3) -> Vec3{
        v / v.length()
    }
}

// operator overloading
impl ops::Neg for Vec3 {
    type Output = Vec3;

    fn neg(self) -> Self::Output {
        Vec3{e:[
            -self.e[0],
            -self.e[1],
            -self.e[2],
        ]}
    }
}

impl ops::Index<usize> for Vec3 {
    type Output = f32;

    fn index(&self, index: usize) -> &Self::Output {
        &self.e[index]
    }
}

impl ops::IndexMut<usize> for Vec3 {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.e[index]
    }
}

impl ops::AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        *self = Self{e:[
            self.e[0] + rhs.e[0],
            self.e[1] + rhs.e[1],
            self.e[2] + rhs.e[2],
        ]}
    }
}

impl ops::MulAssign<f32> for Vec3 {
    fn mul_assign(&mut self, rhs: f32) {
        (*self).e[0] *= rhs;
        (*self).e[1] *= rhs;
        (*self).e[2] *= rhs;
    }
}

impl ops::DivAssign<f32> for Vec3 {
    fn div_assign(&mut self, rhs: f32) {
        (*self) *= 1.0/rhs;
    }
}

impl ops::Add for Vec3{
    type Output = Vec3;

    fn add(self, rhs: Self) -> Self::Output {
        Vec3{ e:[
            self[0] + rhs[0],
            self[1] + rhs[1],
            self[2] + rhs[2],
        ]}
    }
}

impl ops::Sub for Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: Self) -> Self::Output {
       self + -rhs
    }
}

impl ops::Mul<f32> for Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: f32) -> Self::Output {
        Vec3 {e:[
            rhs * self.e[0],
            rhs * self.e[1],
            rhs * self.e[2],
        ] }
    }
}

impl ops::Mul<Vec3> for f32 {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Self::Output {
        rhs * self
    }
}

impl ops::Div<f32> for Vec3 {
    type Output = Vec3;

    fn div(self, rhs: f32) -> Self::Output {
        self * (1.0/rhs)
    }
}

// display
impl std::fmt::Display for Vec3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({} {} {})", self.e[0], self.e[1], self.e[2])
    }
}

impl Eq for Vec3 {}

// tests
#[cfg(test)]
mod tests{
    use  super::*;
    
    #[test]
    fn creation_empty(){
        let vec = Vec3::new();
        assert_eq!(vec.e ,[0.0, 0.0, 0.0]);
    }
    #[test]
    fn creation_value(){
        let vec = Vec3::from_values(1.0, 2.0, 3.0);
        assert_eq!(vec.e, [1.0, 2.0, 3.0]);
    }
    
    #[test]
    fn acessor_x(){
        assert_eq!(Vec3::from_values(1.0, 2.0, 3.0).x(), 1.0)
    }
    #[test]
    fn acessor_y(){
        assert_eq!(Vec3::from_values(1.0, 2.0, 3.0).y(), 2.0)
    }
    #[test]
    fn acessor_z(){
        assert_eq!(Vec3::from_values(1.0, 2.0, 3.0).z(), 3.0)
    }

    #[test]
    fn negative(){
        assert_eq!(-Vec3::from_values(1.0, 2.0, 3.0), Vec3::from_values(-1.0, -2.0, -3.0))
    }
    #[test]
    fn indexing(){
        let vec = Vec3::from_values(0.0, 1.0, 2.0);
        for i in 0..3{
            assert_eq!(vec[i], i as f32);
        }
    }
    #[test]
    fn mut_indexing(){
        let mut vec = Vec3::new();
        for i in 0..3{
            vec[i] = i as f32;
        }
        for i in 0..3{
            assert_eq!(vec[i], i as f32);
        }
    }
    #[test]
    fn add_assign(){
        let mut vec = Vec3::from_values(1.0, 2.0, 3.0);
        vec += Vec3::from_values(1.0, 2.0, 3.0);
        assert_eq!(vec, Vec3::from_values(2.0, 4.0, 6.0));
    }
    #[test]
    fn mul_assign(){
        let mut vec = Vec3::from_values(1.0, 2.0, 3.0);
        vec *= 4.0;
        assert_eq!(vec, Vec3::from_values(4.0, 8.0, 12.0));
    }
    #[test]
    fn div_assign(){
        let mut vec = Vec3::from_values(2.0, 4.0, 6.0);
        vec /= 2.0;
        assert_eq!(vec, Vec3::from_values(1.0, 2.0, 3.0))
    }

    #[test]
    fn length(){
        let vec = Vec3::from_values(1.0, 1.0, 1.0);
        assert_eq!(vec.length(), f32::sqrt(3.0));
    }
    #[test]
    fn length_squared(){
        let vec = Vec3::from_values(1.0, 1.0, 1.0);
        assert_eq!(vec.length_squared(), 3.0);
    }

    #[test]
    fn add(){
        assert_eq!(
            Vec3::from_values(1.0, 2.0, 3.0) + Vec3::from_values(1.0, 2.0, 3.0),
            Vec3::from_values(2.0, 4.0, 6.0)
        )
    }
    #[test]
    fn subtract(){
        assert_eq!(
            Vec3::from_values(1.0, 2.0, 3.0) - Vec3::from_values(1.0, 2.0, 3.0),
            Vec3::new()
        )
    }
    #[test]
    fn multiply(){
        assert_eq!(Vec3::from_values(1.0, 2.0, 3.0) * 2.0, Vec3::from_values(2.0, 4.0, 6.0));
        assert_eq!(2.0 * Vec3::from_values(1.0, 2.0, 3.0) , Vec3::from_values(2.0, 4.0, 6.0));
    }
    #[test]
    fn divide(){
        assert_eq!(Vec3::from_values(2.0, 4.0, 6.0) / 2.0, Vec3::from_values(1.0, 2.0, 3.0));
    }

    #[test]
    fn dot(){
        let vec = Vec3::from_values(1.0, 2.0, 3.0);
        assert_eq!(Vec3::dot(vec, vec), 14.0);
    }

    #[test]
    fn cross(){
        let vec = Vec3::from_values(1.0, 2.0, 3.0);
        assert_eq!(Vec3::cross(vec, vec), Vec3::new());
    }
    #[test]
    fn unit_vector(){
        let vec = Vec3::from_values(1.0, 2.0, 3.0);
        let vec_norm = Vec3::unit_vector(vec);
        assert_eq!(vec_norm.length(), 1.0)
    }

}
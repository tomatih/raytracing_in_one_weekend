use std::ops;

#[derive(Debug, PartialEq)]
struct Vec3{
    e: [f32; 3],
}

// aliases
type Point3 = Vec3;
type Color = Vec3;

impl Vec3 {
    // constructors
    fn new() -> Self{
        Vec3 { e: [0.0, 0.0, 0.0] }
    }
    fn from_values(x:f32, y:f32, z:f32) -> Self{
        Vec3 { e: [x, y, z] }
    }
    // accessors
    fn x(self) -> f32 { self.e[0] }
    fn y(self) -> f32 { self.e[1] }
    fn z(self) -> f32 { self.e[2] }
    // lenght
    fn length_squared(self) -> f32{
        self.e[0] * self.e[0] + self.e[1] * self.e[1] + self.e[2] * self.e[2]
    }
    fn length(self) -> f32{
        f32::sqrt(self.length_squared())
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
    fn test_length(){
        let vec = Vec3::from_values(1.0, 1.0, 1.0);
        assert_eq!(vec.length(), f32::sqrt(3.0));
    }
    #[test]
    fn test_length_squared(){
        let vec = Vec3::from_values(1.0, 1.0, 1.0);
        assert_eq!(vec.length_squared(), 3.0);
    }
    
}
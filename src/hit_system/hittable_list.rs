use crate::materials::Material;

use super::{HitRecord, Hittable};

pub struct HittableList {
    objects: Vec<Box<dyn Hittable + Sync>>,
    materials: Vec<Box<dyn Material + Sync>>,
}

impl HittableList {
    pub fn new() -> Self {
        HittableList {
            objects: Vec::new(),
            materials: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.objects.clear();
        self.materials.clear()
    }

    pub fn add_object(&mut self, object: Box<dyn Hittable + Sync>) {
        self.objects.push(object);
    }

    pub fn add_material(&mut self, material: Box<dyn Material + Sync>) {
        self.materials.push(material);
    }

    pub fn get_last_material_index(&self) -> usize {
        self.materials.len() - 1
    }

    pub fn get_material(&self, index: usize) -> &Box<dyn Material + Sync> {
        &self.materials[index]
    }
}

impl Hittable for HittableList {
    fn hit(&self, ray: &crate::ray::Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        // inintial search conditions
        let mut hit_anything = None;
        let mut closest_so_far = t_max;
        // go though objects
        for object in self.objects.iter() {
            // if found a hit
            if let Some(hit_record) = object.hit(ray, t_min, closest_so_far) {
                // update memory
                closest_so_far = hit_record.t;
                hit_anything = Some(hit_record);
            }
        }
        // return results
        hit_anything
    }
}

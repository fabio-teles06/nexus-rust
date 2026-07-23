use glam::Vec4;
use std::{collections::HashMap, marker::PhantomData};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetId(pub u64);

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Handle<T> { id: AssetId, marker: PhantomData<fn() -> T> }
impl<T> Copy for Handle<T> {}
impl<T> Clone for Handle<T> { fn clone(&self) -> Self { *self } }
impl<T> Handle<T> { pub fn id(self) -> AssetId { self.id } }

#[derive(Debug)]
pub struct Assets<T> { next_id: u64, values: HashMap<AssetId, T> }
impl<T> Default for Assets<T> { fn default() -> Self { Self { next_id: 1, values: HashMap::new() } } }
impl<T> Assets<T> {
    pub fn add(&mut self, asset: T) -> Handle<T> {
        let id = AssetId(self.next_id);
        self.next_id += 1;
        self.values.insert(id, asset);
        Handle { id, marker: PhantomData }
    }
    pub fn get(&self, handle: Handle<T>) -> Option<&T> { self.values.get(&handle.id) }
    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> { self.values.get_mut(&handle.id) }
    pub fn remove(&mut self, handle: Handle<T>) -> Option<T> { self.values.remove(&handle.id) }
    pub fn len(&self) -> usize { self.values.len() }
    pub fn is_empty(&self) -> bool { self.values.is_empty() }
}

#[derive(Debug, Clone)]
pub enum MeshAsset { Cube, Plane }

#[derive(Debug, Clone)]
pub struct MaterialAsset { pub base_color: Vec4 }
impl MaterialAsset { pub fn new(base_color: Vec4) -> Self { Self { base_color } } }

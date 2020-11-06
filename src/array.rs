use crate::{GuillotineAllocator, ShelfAllocator, Rectangle, Size};
use crate::tiled::SlabAllocatorRegion;
use crate::{point2, size2};
use crate::free_list::*;

#[derive(Copy, Clone, Debug)]
pub struct ArrayAllocId {
    id: u32,
    allocator_kind: AllocatorKind,
    allocator_idx: FreeListHandle,
    region_idx: RegionIndex,
}

pub struct ArrayAllocation {
    pub rectangle: Rectangle,
    pub layer: u16,
    pub id: ArrayAllocId,
}

struct RegionInfo {
    rectangle: Rectangle,
    layer: u16,
    region_idx: RegionIndex,
}

type AllocatorIndex = usize;

pub struct ArrayAtlasAllocator {
    guillotines: FreeList<(GuillotineAllocator, RegionInfo)>,
    horizontal_shelves: FreeList<(ShelfAllocator, RegionInfo)>,
    // 16 32 64 256
    tiles: FreeList<(SlabAllocatorRegion, RegionInfo)>,
    size: Size,
    // Width and height of the regions in device pixels.
    region_size: u16,
    regions: Regions,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum AllocatorKind {
    Guillotine,
    HorizontalShelf,
    Tiled(u8),
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum RegionSize {
    Small,
    Large,
}

impl ArrayAtlasAllocator {
    pub fn new(size: Size) -> Self {
        ArrayAtlasAllocator {
            regions: Regions {
                regions: Vec::new(),
                layout: RegionLayout {
                    width: 2,
                    height: 2,
                    large_region_size: 2,
                },
            },
            region_size: size.width as u16 / 2,
            guillotines: FreeList::new(),
            horizontal_shelves: FreeList::new(),
            tiles: FreeList::new(),
            size,
        }
    }

    /// Allocate a rectangle in the atlas.
    pub fn allocate(&mut self, size: Size) -> Option<ArrayAllocation> {
        let kind = self.select_allocator_kind(size);

        match kind {
            AllocatorKind::Guillotine => {
                for (allocator_idx, (ref mut allocator, ref mut region)) in self.guillotines.iter_mut_with_handles() {
                    if let Some(alloc) = allocator.allocate(size) {
                        return Some(ArrayAllocation {
                            id: ArrayAllocId {
                                id: alloc.id.serialize(),
                                region_idx: region.region_idx,
                                allocator_idx,
                                allocator_kind: kind,
                            },
                            layer: region.layer,
                            rectangle: alloc.rectangle.translate(region.rectangle.min.to_vector()),
                        });
                    }
                }

                let allocator_idx = self.add_allocator(kind)?;
                let (ref mut allocator, ref mut region) = self.guillotines[allocator_idx];
                return allocator.allocate(size).map(|alloc| ArrayAllocation {
                    id: ArrayAllocId {
                        id: alloc.id.serialize(),
                        region_idx: region.region_idx,
                        allocator_idx,
                        allocator_kind: kind,
                    },
                    layer: region.layer,
                    rectangle: alloc.rectangle.translate(region.rectangle.min.to_vector()),
                });
            }
            AllocatorKind::Tiled(tile_size) => {
                //for (allocator_idx, (ref mut allocator, ref mut region)) in self.tiles.iter_mut_with_handles() {
                //    if allocator.
                //}

                unimplemented!();
            }
            _ => {
                unimplemented!();
            }
        }
    }

    /// Deallocate a rectangle in the atlas.
    pub fn deallocate(&mut self, id: ArrayAllocId) {
        unimplemented!();
    }

    fn select_allocator_kind(&self, size: Size) -> AllocatorKind {
        let max = size.width.max(size.height);
        let next_pow2 = (max as u32).next_power_of_two();

        unimplemented!();
    }

    fn add_allocator(&mut self, kind: AllocatorKind) -> Option<FreeListHandle> {
        let region_size = match kind {
            AllocatorKind::Guillotine => RegionSize::Large,
            _ => RegionSize::Small,
        };

        let large_region_size = self.regions.layout.large_region_size;
        let (n_regions, size) = match region_size {
            RegionSize::Small => (1, self.region_size),
            RegionSize::Large => ((large_region_size * large_region_size) as u8, self.region_size * large_region_size),
        };

        let region_idx = if let Some(idx) = self.regions.allocate_region(n_regions, FreeListHandle::NONE) {
            idx
        } else {
            return None;
        };

        let (x, y, layer) = self.regions.layout.position_for_region_idx(region_idx);
        let min = point2(x as i32, y as i32);
        let size = size2(size as i32, size as i32);
        let max = min + size.to_vector();
        let rectangle = Rectangle { min, max };

        let region_info = RegionInfo {
            rectangle,
            layer,
            region_idx,
        };

        let allocator_idx = match kind {
            AllocatorKind::Guillotine => {
                self.guillotines.add_with_value((GuillotineAllocator::new(size), region_info))
            }
            AllocatorKind::HorizontalShelf => {
                self.horizontal_shelves.add_with_value((ShelfAllocator::new(size), region_info))
            }
            AllocatorKind::Tiled(tile_size) => {
                let size_in_slots = (self.region_size / tile_size as u16).min(255) as i32;
                self.tiles.add_with_value((
                    SlabAllocatorRegion::new(size2(size_in_slots, size_in_slots)),
                    region_info
                ))
            }
        };

        self.regions.set_allocator(region_idx, allocator_idx);

        Some(allocator_idx)
    }

}

struct RegionLayout {
    width: u16,
    height: u16,
    large_region_size: u16,
}

impl RegionLayout {
    fn regions_per_layer(&self) -> u16 {
        self.width * self.height
    }

    fn position_for_region_idx(&self, region_idx: RegionIndex) -> (u16, u16, u16) {
        let region_idx = region_idx.0;
        let layer = region_idx / self.regions_per_layer();
        let index_in_layer = region_idx % self.regions_per_layer();
        let large_regions_in_wdith = self.width / self.large_region_size;
        let large_region_x = region_idx % large_regions_in_wdith;
        let large_region_y = region_idx / large_regions_in_wdith;
        let x = large_region_x + index_in_layer % self.large_region_size;
        let y = large_region_y + index_in_layer / self.large_region_size;

        (x, y, layer)
    }
}

struct Region {
    allocator: FreeListHandle,
    size: u8,
}


#[derive(Copy, Clone, Debug, PartialEq)]
struct RegionIndex(u16);

pub struct Regions {
    regions: Vec<Region>,
    layout: RegionLayout,
}

impl Regions {
    fn allocate_region(&mut self, size: u8, allocator: FreeListHandle) -> Option<RegionIndex> {
        let step = size as usize;
        'outer: for i in (0..self.regions.len()).step_by(step) {
            if self.regions[i].allocator.is_some() {
                continue;
            }

            for j in 1..step {
                if self.regions[i+j].allocator.is_some() {
                    continue 'outer;
                }
            }

            for j in 0..step {
                self.regions[i+j].allocator = allocator;
            }
            self.regions[i].size = size;

            return Some(RegionIndex(i as u16));
        }

        return None;
    }

    fn set_allocator(&mut self, region: RegionIndex, allocator: FreeListHandle) {
        let n = self.regions[region.0 as usize].size as usize;
        for i in 0..n {
            self.regions[region.0 as usize + i].allocator = allocator;
        }
    }

    fn deallocate_region(&mut self, index: usize) {
        for i in 0..(self.regions[index].size as usize) {
            self.regions[index + i] = Region {
                allocator: FreeListHandle::NONE,
                size: 1,
            }
        }
    }

    fn add_regions(&mut self, count: usize) {
        for i in 0..count {
            self.regions.push(Region{
                allocator: FreeListHandle::NONE,
                size: 1,
            });
        }
    }

    fn shrink(&mut self) {
        while self.regions.last().map(|region| region.allocator.is_none()) == Some(true) {
            self.regions.pop();
        }
    }

    fn shrink_to_power_of_two(&mut self) {
        self.shrink();

        if !self.regions.len().is_power_of_two() {
            self.regions.push(Region{
                allocator: FreeListHandle::NONE,
                size: 1,
            });
        }
    }

    fn shrink_to_multiple_of(&mut self, multiple: usize) {
        self.shrink();

        if self.regions.len() % multiple != 0 {
            self.regions.push(Region{
                allocator: FreeListHandle::NONE,
                size: 1,
            });
        }
    }
}

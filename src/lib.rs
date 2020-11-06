#![deny(unconditional_recursion)]

#[macro_use]
#[cfg(feature = "serialization")]
pub extern crate serde;
pub extern crate euclid;
pub extern crate guillotiere;

pub mod free_list;
pub mod etagere;
//pub mod array;
pub mod tiled;

pub use euclid::{vec2, point2, size2};

pub type Point = euclid::default::Point2D<i32>;
pub type Size = euclid::default::Size2D<i32>;
pub type Rectangle = euclid::default::Box2D<i32>;

pub type GuillotineAllocator = guillotiere::AtlasAllocator;
pub type ShelfAllocator = etagere::AtlasAllocator;
pub use crate::tiled::TiledAllocator;



/// ID referring to an allocated rectangle within a given texture.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AllocId(pub(crate) u32);

/// ID of a texture.
#[repr(C)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(pub(crate) u32);

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Allocation {
    pub id: AllocId,
    pub texture: TextureId,
    pub rectangle: Rectangle,
}

#[repr(C)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Handle {
    texture: TextureId,
    alloc: AllocId,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TextureIdGenerator {
    next: u32,
}

impl TextureIdGenerator {
    pub fn new() -> Self {
        TextureIdGenerator {
            next: 1,
        }
    }

    pub fn generate(&mut self) -> TextureId {
        let id = TextureId(self.next);
        self.next += 1;

        id
    }
}

pub trait AtlasAllocator {
    type Config;

    fn new(size: Size, config: &Self::Config) -> Self;

    /// Allocate a rectangle in the atlas.
    fn allocate(&mut self, size: Size) -> Option<(AllocId, Rectangle)>;

    /// Deallocate a rectangle in the atlas.
    fn deallocate(&mut self, id: AllocId);

    /// The total size of the atlas.
    fn size(&self) -> Size;

    fn clear(&mut self);

    fn is_empty(&self) -> bool;

    fn dump_into_svg(&self, rect: Option<&Rectangle>, output: &mut dyn std::io::Write) -> std::io::Result<()>;
}

impl AtlasAllocator for guillotiere::AtlasAllocator {
    type Config = guillotiere::AllocatorOptions;

    fn new(size: Size, options: &guillotiere::AllocatorOptions) -> Self {
        guillotiere::AtlasAllocator::with_options(size, options)
    }

    fn allocate(&mut self, size: Size) -> Option<(AllocId, Rectangle)> {
        self.allocate(size).map(|allocation| (
            AllocId(allocation.id.serialize()),
            allocation.rectangle,
        ))
    }

    fn deallocate(&mut self, id: AllocId) {
        self.deallocate(guillotiere::AllocId::deserialize(id.0));
    }

    fn size(&self) -> Size {
        self.size()
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn dump_into_svg(&self, rect: Option<&Rectangle>, output: &mut dyn std::io::Write) -> std::io::Result<()> {
        guillotiere::dump_into_svg(self, rect, output)
    }
}

impl AtlasAllocator for etagere::AtlasAllocator {
    type Config = etagere::AllocatorOptions;

    fn new(size: Size, options: &etagere::AllocatorOptions) -> Self {
        etagere::AtlasAllocator::with_options(size, options)
    }

    fn allocate(&mut self, size: Size) -> Option<(AllocId, Rectangle)> {
        self.allocate(size).map(|(alloc, rect)| (
            AllocId(alloc.serialize()),
            rect,
        ))
    }

    fn deallocate(&mut self, id: AllocId) {
        self.deallocate(etagere::AllocId::deserialize(id.0));
    }

    fn size(&self) -> Size {
        self.size()
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn dump_into_svg(&self, rect: Option<&Rectangle>, output: &mut dyn std::io::Write) -> std::io::Result<()> {
        etagere::dump_into_svg(self, rect, output)
    }
}

impl AtlasAllocator for crate::tiled::TiledAllocator {
    type Config = (crate::tiled::TileSizes, Size);

    fn new(size: Size, options: &(crate::tiled::TileSizes, Size)) -> Self {
        crate::tiled::TiledAllocator::new(
            size,
            options.0,
            &[crate::tiled::TiledAllocatorOptions { region_size: options.1 }],
        )
    }

    fn allocate(&mut self, size: Size) -> Option<(AllocId, Rectangle)> {
        self.allocate(size).map(|alloc| (alloc.id, alloc.rectangle))
    }

    fn deallocate(&mut self, id: AllocId) {
        self.deallocate(id);
    }

    fn size(&self) -> Size {
        self.size()
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn dump_into_svg(&self, rect: Option<&Rectangle>, output: &mut dyn std::io::Write) -> std::io::Result<()> {
        tiled::dump_into_svg(self, rect, output)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Texture<Allocator> {
    id: TextureId,
    allocator: Allocator,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AllocatorList<Allocator: AtlasAllocator> {
    textures: Vec<Texture<Allocator>>,
    size: Size,
    ids: TextureIdGenerator,
    config: Allocator::Config,
}

impl<Allocator: AtlasAllocator> AllocatorList<Allocator> {
    pub fn new(size: Size, config: Allocator::Config) -> Self {
        AllocatorList {
            textures: Vec::new(),
            size,
            config,
            ids: TextureIdGenerator::new(),
        }
    }

    pub fn allocate(&mut self, size: Size) -> Option<(Handle, Rectangle)> {
        if size.width > self.size.width || size.height > self.size.height {
            return None;
        }

        for texture in &mut self.textures {
            if let Some((alloc, rectangle)) = texture.allocator.allocate(size) {
                return Some((
                    Handle { texture: texture.id, alloc },
                    rectangle
                ));
            }
        }

        self.textures.push(Texture {
            id: self.ids.generate(),
            allocator: Allocator::new(self.size, &self.config),
        });

        let texture = self.textures.last_mut().unwrap();
        texture.allocator.allocate(size).map(|(alloc, rectangle)| (
            Handle { texture: texture.id, alloc },
            rectangle,
        ))
    }

    pub fn deallocate(&mut self, handle: Handle) {
        let mut empty_index = None;
        for (idx, texture) in self.textures.iter_mut().enumerate() {
            if texture.id != handle.texture {
                continue;
            }
            texture.allocator.deallocate(handle.alloc);

            if texture.allocator.is_empty() {
                empty_index = Some(idx)
            }

            break;
        }

        if let Some(idx) = empty_index {
            self.textures.swap_remove(idx);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    pub fn num_textures(&self) -> usize {
        self.textures.len()
    }

    pub fn dump_svg(&self, output: &mut dyn std::io::Write) -> std::io::Result<()> {
        use svg_fmt::*;

        let size = 512.0;
        let spacing = 10.0;

        let svg_w = 2.0 * spacing + size * self.textures.len() as f32;
        let svg_h = 2.0 * spacing + size;

        writeln!(output, "{}", BeginSvg { w: svg_w, h: svg_h } )?;

        // Background.
        writeln!(output,
            "    {}",
            rectangle(0.0, 0.0, svg_w, svg_h)
                .inflate(1.0, 1.0)
                .fill(rgb(30, 30, 30))
        )?;

        let mut rect = crate::Rectangle {
            min: point2(spacing, spacing).to_i32(),
            max: point2(size + spacing, size + spacing).to_i32(),
        };

        for texture in &self.textures {
            texture.allocator.dump_into_svg(Some(&rect), output)?;
            rect = rect.translate(vec2(size as i32 + spacing as i32, 0));
        }

        writeln!(output, "{}", EndSvg)
    }
}

//const SLABS_TEST: &'static[&'static [(i32, i32)]] = &[
//    &[(8,8), (16,16), (32, 32), (64,64), (128,128), (256,256)],
//    &[(8,14), (12, 14), (12,16), (14,14), (14,16)],
//];

const SIZES: &'static[i32] = &[4, 6, 8, 10, 12, 14, 16, 18, 20, 24, 32, 40, 48, 56, 64, 80, 96, 128, 256];
const N: usize = SIZES.len();

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StatsRecorder {
    widths: Vec<u64>,
    heights: Vec<u64>,
    height_width: Vec<Vec<u64>>,
    waste: Vec<Vec<i32>>,
    num_allocs: u64,
}

impl StatsRecorder {
    pub fn new() -> Self {
        StatsRecorder {
            widths: vec![0; N],
            heights: vec![0; N],
            height_width: vec![vec![0; N]; N],
            waste: vec![vec![0; N]; N],
            num_allocs: 0,
        }
    }

    fn size_index(size: i32) -> usize {
        for i in 0..N {
            if SIZES[i] >= size {
                return i
            }
        }

        SIZES.len() - 1
    }

    pub fn allocate(&mut self, size: Size) {
        self.num_allocs += 1;

        let w = Self::size_index(size.width);
        let h = Self::size_index(size.height);
        self.widths[w] += 1;
        self.heights[h] += 1;
        self.height_width[h][w] += 1;
        let waste = SIZES[w] * SIZES[h] - size.width * size.height;
        self.waste[h][w] += waste;
    }

    pub fn print(&self) {
        println!("# Widths:");
        print!("size:\t");
        for size in SIZES {
            print!("\t{:?}", size);
        }
        println!("+");

        print!("count:\t");
        for i in 0..N {
            print!("\t{:?}", self.widths[i]);
        }
        println!("");

        println!("# Heights:");
        print!("size:\t");
        for size in SIZES {
            print!("\t{:?}", size);
        }
        println!("+");

        print!("count:\t");
        for i in 0..N {
            print!("\t{:?}", self.heights[i]);
        }
        println!("");

        print!("\n\n\n\t");

        for size in SIZES {
            print!("\t{:?}", size);
        }
        println!("+\n\t");
        for _ in 0..(N + 1) {
            print!("\t-");
        }
        println!("");
        for h in 0..N {

            print!("{:?}\t|", SIZES[h]);
            for w in 0..N {
                print!("\t{:?}", self.height_width[h][w]);
            }
            println!("");
        }


        print!("\n\nWaste\n\t");

        for size in SIZES {
            print!("\t{:?}", size);
        }
        println!("+\n\t");
        for _ in 0..(N + 1) {
            print!("\t-");
        }
        println!("");
        for h in 0..N {

            print!("{:?}\t|", SIZES[h]);
            for w in 0..N {
                print!("\t{:.2}", self.waste[h][w]);
            }
            println!("");
        }
    }
}

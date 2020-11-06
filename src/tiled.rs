use crate::{Rectangle, Size, size2, Point, point2};
//use crate::free_list::*;

use crate::AllocId;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TiledAllocatorOptions {
    pub region_size: Size,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ArrayAllocation {
    pub id: AllocId,
    pub layer: u16,
    pub rectangle: Rectangle,    
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TiledRegion {
    free_slots: Vec<(u8, u8)>,
    tile_size: Size,
    origin: Point,
    size: Size,
    num_tiles: u16,
    index: u16,
    layer: u16,
}

impl TiledRegion {
    fn allocate(&mut self) -> Option<ArrayAllocation> {
        let slot = self.free_slots.pop()?;

        let x = self.origin.x + slot.0 as i32 * self.tile_size.width;
        let y = self.origin.y + slot.1 as i32 * self.tile_size.height;

        debug_assert_eq!(self.index & 0xFFFF, self.index);

        let id = AllocId(
            self.index as u32 & 0xFFFF
            | (slot.0 as u32) << 16
            | (slot.1 as u32) << 24
        );

        Some(ArrayAllocation {
            id,
            layer: self.layer,
            rectangle: Rectangle {
                min: point2(x, y),
                max: point2(
                    x + self.tile_size.width,
                    y + self.tile_size.height,
                ),
            }
        })
    }

    fn init(&mut self, tile_size: Size) {
        let n_tiles_x = self.size.width / tile_size.width;
        let n_tiles_y = self.size.height / tile_size.height;
        let n_tiles = (n_tiles_x * n_tiles_y) as usize;

        self.tile_size = tile_size;
        self.free_slots.clear();
        self.free_slots.reserve(n_tiles);
        self.num_tiles = n_tiles as u16;

        for y in 0..(n_tiles_y as u8) {
            for x in 0..(n_tiles_x as u8) {
                self.free_slots.push((x, y));
            }
        }

        println!(" == init {:?} tile_size {:?} -> num_tiles: {:?}", self.size, tile_size, n_tiles);
    }

    fn is_empty(&self) -> bool {
        self.free_slots.len() == self.num_tiles as usize
    }

    fn clear(&mut self) {
        self.free_slots.clear();
        self.num_tiles = 0;
        self.tile_size = size2(0, 0);
    }
}


#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TiledAllocator {
    regions: Vec<TiledRegion>,
    size: Size,
    layers: u16,
    tile_sizes: TileSizes,
}

impl TiledAllocator {
    pub fn new(size: Size, tile_sizes: TileSizes, layers: &[TiledAllocatorOptions]) -> Self {

        let mut regions = Vec::new();

        for (layer, options) in layers.iter().enumerate() {
            let regions_x = size.width / options.region_size.width;
            let regions_y = size.height / options.region_size.height;
            regions.reserve((regions_x * regions_y) as usize);

            for y in 0..regions_y {
                for x in 0..regions_x {
                    let index = regions.len() as u16;
                    regions.push(TiledRegion {
                        free_slots: Vec::new(),
                        tile_size: Size::new(0, 0),
                        size: options.region_size,
                        num_tiles: 0,
                        origin: point2(x, y),
                        index,
                        layer: layer as u16,
                    });
                }
            }
        }

        TiledAllocator {
            regions,
            size,
            layers: layers.len() as u16,
            tile_sizes,
        }
    }

    pub fn allocate(&mut self, size: Size) -> Option<ArrayAllocation> {
        println!("allocate {:?}", size);
        let size = self.tile_sizes.get(size)?;

        println!(" - tile size {:?}", size);

        let mut empty_index = None;
        for (idx, region) in self.regions.iter_mut().enumerate() {
            if empty_index.is_none()
                && region.is_empty()
                && region.size.width >= size.width
                && region.size.height >= size.height {
                empty_index = Some(idx);
            }
            if region.tile_size == size {
                if let Some(alloc) = region.allocate() {
                    return Some(alloc);
                }
            }
        }

        println!(" - need to allocate a region {:?}", empty_index);

        if let Some(idx) = empty_index {
            let region = &mut self.regions[idx];
            println!(" init region {:?}", idx);
            region.init(size);
            return region.allocate();
        }

        None
    }

    pub fn deallocate(&mut self, id: AllocId) {
        let region_idx = (id.0 & 0xFFFF) as usize;
        let x = ((id.0 >> 16) & 0xFF) as u8;
        let y = ((id.0 >> 24) & 0xFF) as u8;
        let region = &mut self.regions[region_idx];
        debug_assert!(region.free_slots.len() < region.num_tiles as usize);
        region.free_slots.push((x, y));

        if region.is_empty() {
            println!("region is empty");
            region.free_slots.clear();
            region.tile_size = Size::new(0, 0);
            region.num_tiles = 0;
        }
    }

    pub fn allocate_full_layer(&mut self) -> ArrayAllocation {
        let layer = self.layers;
        let index = self.regions.len() as u16;
        self.regions.push(TiledRegion {
            free_slots: Vec::new(),
            tile_size: Size::new(0, 0),
            size: self.size,
            num_tiles: 1,
            origin: point2(0, 0),
            index,
            layer,
        });

        ArrayAllocation {
            id: AllocId(index as u32),
            layer,
            rectangle: Rectangle {
                min: point2(0, 0),
                max: point2(0, 0).add_size(&self.size),
            },

        }
    }

    pub fn num_layers(&self) -> u16 {
        self.layers
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        for region in &self.regions {
            if !region.is_empty() {
                return false;
            }
        }

        true
    }

    pub fn clear(&mut self) {
        for region in &mut self.regions {
            region.clear();
        }
    }
}

#[derive(Copy, Clone, Debug,)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TileSizes {
    WrDefault,
    WrGlyphs,
}

impl TileSizes {
    pub fn get(&self, size: Size) -> Option<Size> {
        match *self {
            TileSizes::WrDefault => wr_default_tile_size(size),
            TileSizes::WrGlyphs => wr_glyphs_tile_size(size),
        }
    }
}

pub fn wr_default_tile_size(size: Size) -> Option<Size> {
    fn quantize_dimension(size: i32) -> Option<i32> {
        match size {
            0 => None,
            1..=16 => Some(16),
            17..=32 => Some(32),
            33..=64 => Some(64),
            65..=128 => Some(128),
            129..=256 => Some(256),
            257..=512 => Some(512),
            _ => None,
        }
    }

    let w = quantize_dimension(size.width)?;
    let h = quantize_dimension(size.height)?;

    let (w, h) = match (w, h) {
        // Special cased rectangular tiles.
        (512, 1..=64)  => (512, 64),
        (512, 128) => (512, 128),
        (512, 256) => (512, 256),
        (1..=64, 512) => (64, 512),
        (128, 512) => (128, 512),
        (256, 512) => (256, 512),
        // default to square tiles
        (w, h) => {
            let square_size = std::cmp::max(w, h);
            (square_size, square_size)
        }
    };

    Some(Size::new(w, h))
}

pub fn wr_glyphs_tile_size(size: Size) -> Option<Size> {
    fn quantize_dimension(size: i32) -> Option<i32> {
        match size {
            0 => None,
            1..=8 => Some(8),
            9..=16 => Some(16),
            17..=32 => Some(32),
            33..=64 => Some(64),
            65..=128 => Some(128),
            129..=256 => Some(256),
            257..=512 => Some(512),
            _ => None,
        }
    }

    let w = quantize_dimension(size.width)?;
    let h = quantize_dimension(size.height)?;

    let (w, h) = match (w, h) {
        // Special cased rectangular tiles.
        (8, 16)  => (8, 16),
        (16, 32) => (16, 32),
        // default to square tiles
        (w, h) => {
            let square_size = std::cmp::max(w, h);
            (square_size, square_size)
        }
    };

    Some(Size::new(w, h))
}



use svg_fmt::*;

/// Dump a visual representation of the atlas in SVG format.
pub fn dump_svg(atlas: &TiledAllocator, output: &mut dyn std::io::Write) -> std::io::Result<()> {
    let (layers_in_x, layers_in_y) = arrange_layers(atlas.num_layers() as usize);
    let spacing = 5.0;

    writeln!(
        output,
        "{}",
        BeginSvg {
            w: (atlas.size().width as f32 + spacing) * layers_in_x as f32 - spacing,
            h: (atlas.size().height as f32 + spacing) * layers_in_y as f32 - spacing,
        }
    )?;

    dump_into_svg(atlas, None, output)?;

    writeln!(output, "{}", EndSvg)
}

/// Dump a visual representation of the atlas in SVG, omitting the beginning and end of the
/// SVG document, so that it can be included in a larger document.
///
/// If a rectangle is provided, translate and scale the output to fit it.
pub fn dump_into_svg(atlas: &TiledAllocator, rect: Option<&Rectangle>, output: &mut dyn std::io::Write) -> std::io::Result<()> {
    let layer_width = atlas.size().width as f32;
    let layer_height = atlas.size().height as f32; 

    let (layers_in_x, layers_in_y) = arrange_layers(atlas.num_layers() as usize);

    let spacing = 5.0;
    let (sx, sy, x0, y0) = if let Some(rect) = rect {
        let n_layers = layers_in_x.max(layers_in_y) as f32; 
        (
            rect.size().width as f32 / ((layer_width as f32 + spacing) * n_layers - spacing),
            rect.size().height as f32 / ((layer_height as f32 + spacing) * n_layers - spacing),
            rect.min.x as f32,
            rect.min.y as f32,
        )
    } else {
        (1.0, 1.0, 0.0, 0.0)        
    };

    let spacing_x = spacing * sx;
    let spacing_y = spacing * sy;

    let layer_width = layer_width * sx;
    let layer_height = layer_height * sy; 

    for region in &atlas.regions {
        let region_width = region.size.width as f32 * sx;
        let region_height = region.size.height as f32 * sy;

        let layer_x = x0 +(region.layer as usize % layers_in_x) as f32 * (layer_width + spacing_x);
        let layer_y = y0 +(region.layer as usize / layers_in_x) as f32 * (layer_height + spacing_y);
        let region_x = layer_x + region.origin.x as f32 * region_width;
        let region_y = layer_y + region.origin.y as f32 * region_height;

        let slot_width = region.tile_size.width as f32 * sx;
        let slot_height = region.tile_size.height as f32 * sy;

        if !region.is_empty() {
            let n_tiles_x = region.size.width / region.tile_size.width;
            let n_tiles_y = region.size.height / region.tile_size.height;

            // First pretend all slots are allocated and overwrite free slots
            // with gray rectangles.
            for y in 0..n_tiles_y {
                let y = region_y + y as f32 * region.tile_size.height as f32 * sy;
                for x in 0..n_tiles_x {
                    let x = region_x + x as f32 * region.tile_size.width as f32 * sx;
                    writeln!(
                        output,
                        r#"    {}"#,
                        rectangle(x, y, slot_width, slot_height)
                            .fill(rgb(70, 70, 180))
                            .stroke(Stroke::Color(black(), 1.0))
                    )?;
                }
            }

            for &(x, y) in &region.free_slots {
                let x = region_x + x as f32 * region.tile_size.width as f32 * sx;
                let y = region_y + y as f32 * region.tile_size.height as f32 * sy;
                writeln!(
                    output,
                    r#"    {}"#,
                    rectangle(x, y, slot_width, slot_height)
                        .fill(rgb(50, 50, 50))
                        .stroke(Stroke::Color(black(), 1.0))
                )?;
            }
        } else {
            writeln!(
                output,
                r#"    {}"#,
                rectangle(region_x, region_y, region_width, region_height)
                    .fill(rgb(40, 40, 40))
                    .stroke(Stroke::Color(black(), 1.0))
            )?;
        }
    }

    Ok(())
}

fn arrange_layers(num_layers: usize) -> (usize, usize) {
    let mut layers_in_x = num_layers;
    while layers_in_x - 1 > num_layers / layers_in_x {
        layers_in_x -= 1;
    }
    let layers_in_y = num_layers / layers_in_x;

    (layers_in_x, layers_in_y)
}

#[test]
fn simple_1() {
    let mut atlas = TiledAllocator::new(size2(512, 512), TileSizes::WrDefault, &[
        TiledAllocatorOptions { region_size: size2(32, 32) },
        TiledAllocatorOptions { region_size: size2(64, 64) },
        TiledAllocatorOptions { region_size: size2(64, 64) },
        TiledAllocatorOptions { region_size: size2(256, 256) },
    ]);

    assert_eq!(atlas.num_layers(), 4);

    let a = atlas.allocate(size2(10, 10)).unwrap();
    let b = atlas.allocate(size2(60, 50)).unwrap();
    let c = atlas.allocate(size2(10, 10)).unwrap();
    let d = atlas.allocate(size2(256, 256)).unwrap();

    atlas.deallocate(a.id);
    atlas.deallocate(b.id);
    atlas.deallocate(c.id);
    atlas.deallocate(d.id);

    assert!(atlas.is_empty());
}

#[test]
fn simple_2() {
    let mut atlas = TiledAllocator::new(size2(64, 64), TileSizes::WrDefault, &[
        TiledAllocatorOptions { region_size: size2(32, 32) },
        TiledAllocatorOptions { region_size: size2(32, 32) },
    ]);

    atlas.allocate(size2(16, 16)).unwrap();
    atlas.allocate(size2(32, 32)).unwrap();
    atlas.allocate(size2(16, 16)).unwrap();
    atlas.allocate(size2(16, 16)).unwrap();
    atlas.allocate(size2(32, 32)).unwrap();
    let a = atlas.allocate(size2(16, 16)).unwrap();
    atlas.allocate(size2(32, 32)).unwrap();

    let b = atlas.allocate(size2(16, 16)).unwrap();
    let c = atlas.allocate(size2(16, 16)).unwrap();

    atlas.deallocate(b.id);
    atlas.deallocate(c.id);

    atlas.deallocate(a.id);

    atlas.allocate(size2(16, 16)).unwrap();

    //dump_svg(&atlas, &mut std::fs::File::create("test8.svg").expect("!!"));
}

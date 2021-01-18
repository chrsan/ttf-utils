use structopt::StructOpt;

fn main() {
    let opt = Opt::from_args();
    let font_data = std::fs::read(&opt.font_file).unwrap();
    let face = ttf_parser::Face::from_slice(&font_data, opt.face_index).unwrap();
    let glyph_id = face.glyph_index(opt.character).unwrap();
    let mut outline = ttf_utils::Outline::new(&face, glyph_id).unwrap();
    if opt.embolden {
        outline.embolden(20.0);
    }

    if opt.oblique {
        outline.oblique(0.25);
    }

    println!("bbox: {:?}", outline.bbox());
    let mut printer = OutlinePrinter;
    outline.emit(&mut printer);
}

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short, long, default_value = "0")]
    face_index: u32,

    #[structopt(short, long, default_value = "C")]
    character: char,

    #[structopt(short, long)]
    embolden: bool,

    #[structopt(short, long)]
    oblique: bool,

    #[structopt(name = "FONT_FILE", parse(from_os_str))]
    font_file: std::path::PathBuf,
}

struct OutlinePrinter;

impl ttf_parser::OutlineBuilder for OutlinePrinter {
    fn move_to(&mut self, x: f32, y: f32) {
        println!("M {} {}", x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        println!("L {} {}", x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        println!("Q {} {} {} {}", x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        println!("C {} {} {} {} {} {}", x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        println!("Z");
    }
}

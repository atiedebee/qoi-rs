mod qoi;

use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;


fn f_to_buff(fname: &str) -> Vec<u8> {
    let path = Path::new(fname);
    let display = path.display();


    let data = match fs::read(path) {
        Err(reason) => panic!("Couldn't read {}: {}", display, reason),
        Ok(d) => d
    };

    return data;
}

fn serialize_buff(buff: Vec<u8>, fname: &str){
    let path = Path::new(fname);
    let display = path.display();

    let mut f = match File::create(path) {
        Err(reason) => panic!("Couldn't create {}: {}", display, reason),
        Ok(f) => f
    };

    f.write_all(buff.as_slice()).unwrap();

}



fn main() {
    // let data = f_to_buff("test.data");
    let data = include_bytes!("../test.data");
    let mut out = Vec::<u8>::with_capacity( data.len() );

    qoi::compress(data, &mut out, 1920, 1080, 4);

    serialize_buff(out, "testd.qoi");


}

use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write, Read},
    time::Instant,
    env,
};

use ahash::AHashMap;

fn main() {
    let args: Vec<String> = env::args().collect();

    let input_file = File::open(&args[1]).unwrap();
    let input_size = File::metadata(&input_file).unwrap().len();
    let mut g2_compress = G2zWriter::new(
        input_file,
        File::create(format!("{}.g2z", &args[1])).unwrap()
    );
    let now = Instant::now();
    let compressed_size = g2_compress.compress();
    println!(
        "{}ms / {input_size} to {compressed_size} bytes / {:0.2}%",
        now.elapsed().as_millis(),
        ((compressed_size as f32 / input_size as f32) * 100.0)
    );

    let mut g2_decompress = G2zReader::new(
        File::open(format!("{}.g2z", &args[1])).unwrap(),
        File::create(format!("{}.re", &args[1])).unwrap()
    );
    let now = Instant::now();
    g2_decompress.decompress();
    println!("{}ms to decompress", now.elapsed().as_millis());
}

struct G2zWriter<R: Read, W: Write> {
    input: BufReader<R>,
    output: BufWriter<W>,
}

impl<R: Read, W: Write> G2zWriter<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            input: BufReader::new(reader),
            output: BufWriter::new(writer),
        }
    }

    pub fn compress(&mut self) -> usize {
        let mut dict: AHashMap<Vec<u8>, usize> = AHashMap::new();
        let mut file_chunk = Vec::new();
        let mut total_length = 0;
        let mut total_compressed = 0;
        let mut stop = false;
        while !stop {
            let length = self.input.read_until(0x20, &mut file_chunk).unwrap();
            if length == 0 {
                self.output.write_all(&file_chunk).unwrap();
                stop = true;
                continue;
            }
            total_length += length;

            if length < 3 {
                self.output.write_all(&file_chunk).unwrap();
                total_compressed += file_chunk.len();
                file_chunk.clear();
                continue;
            }

            if file_chunk.contains(&0xFF) {
                panic!("Can't compress files which are not strictly ASCII!");
            }

            if !dict.contains_key(&file_chunk) {
                dict.insert(file_chunk.clone(), total_length - length);
                self.output.write_all(&file_chunk).unwrap();
                total_compressed += length;
            } else {
                let pos = dict.get(&file_chunk).unwrap();
                let vint = varint_simd::encode(*pos as u64);

                self.output.write_all(&[0xFF]).unwrap();
                self.output.write_all(&vint.0[..vint.1 as usize]).unwrap();
                total_compressed += &vint.0[..vint.1 as usize].len() + 1;
            }
            file_chunk.clear();
        }

        self.output.flush().unwrap();
        total_compressed
    }
}

struct G2zReader<R: Read, W: Write> {
    input: BufReader<R>,
    output: BufWriter<W>,
}

impl<R: Read, W: Write> G2zReader<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            input: BufReader::new(reader),
            output: BufWriter::new(writer),
        }
    }

    pub fn decompress(&mut self) -> usize {
        let mut total_file: Vec<u8> = Vec::new();
        let mut file_chunk = Vec::new();
        let mut total_length = 0;
        let mut stop = false;
        while !stop {
            let length = self.input.read_until(0xFF, &mut file_chunk).unwrap();
            if length == 0 {
                println!("{:X?}", file_chunk);
                stop = true;
                if !file_chunk.is_empty() {
                    self.output.write_all(&file_chunk).unwrap();
                }
                continue;
            }

            if *file_chunk.last().unwrap() == 0xFF {
                file_chunk.pop().unwrap();
            }
            if total_length > 0 {
                let varint = match varint_simd::decode::<u64>(file_chunk.as_slice()) {
                    Ok(num) => num,
                    Err(_) => {
                        file_chunk.push(0xFF);
                        continue;
                    }
                };

                if varint.1 > file_chunk.len() {
                    file_chunk.push(0xFF);
                    continue;
                }

                file_chunk = file_chunk[varint.1..].to_vec();

                let mut target_data: Vec<u8> = total_file[varint.0 as usize..]
                    .iter()
                    .take_while(|x| **x != 0x20)
                    .copied()
                    .collect();
                target_data.push(0x20);

                //println!("{}: {:X?}", varint.0, file_chunk);

                self.output.write_all(&target_data).unwrap();
                self.output.write_all(&file_chunk).unwrap();

                file_chunk = file_chunk.to_vec();

                total_length += target_data.len();
                total_length += file_chunk.len();
                total_file.append(&mut target_data);
                total_file.append(&mut file_chunk);
            } else {
                self.output.write_all(&file_chunk).unwrap();
                total_length += file_chunk.len();
                total_file.append(&mut file_chunk);
            }
        }
        self.output.flush().unwrap();
        total_length
    }
}

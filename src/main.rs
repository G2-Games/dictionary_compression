use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write, Read}
};

fn main() {
    let mut g2_compress = G2zWriter::new(
        File::open("big.txt").unwrap(),
        File::create("big.txt.g2z").unwrap()
    );
    dbg!(g2_compress.compress());

    let mut g2_decompress = G2zReader::new(
        File::open("big.txt.g2z").unwrap(),
        File::create("big.txt.re").unwrap()
    );
    dbg!(g2_decompress.decompress());
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
        let mut dict: BTreeMap<Vec<u8>, u64> = BTreeMap::new();
        let mut file_chunk = Vec::new();
        let mut total_length = 0;
        let mut total_compressed = 0;
        let mut stop = false;
        while !stop {
            let length = self.input.read_until(0x20, &mut file_chunk).unwrap();
            if length == 0 {
                stop = true;
                continue;
            }
            total_length += length;

            if length < 2 {
                self.output.write_all(&file_chunk).unwrap();
                file_chunk.clear();
                continue;
            }

            if !dict.contains_key(&file_chunk) {
                dict.insert(file_chunk.clone(), total_length as u64 - length as u64);
                self.output.write_all(&file_chunk).unwrap();
                total_compressed += length;
            } else {
                let pos = dict.get(&file_chunk).unwrap();
                let vint = varint_simd::encode(*pos);

                self.output.write_all(&[0xFF]).unwrap();
                self.output.write_all(&vint.0[..vint.1 as usize]).unwrap();
                total_compressed += &vint.0[..vint.1 as usize].len() + 1;
            }
            file_chunk.clear();
            self.output.flush().unwrap();
        }

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
                stop = true;
                continue;
            }

            file_chunk.pop().unwrap();
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
            self.output.flush().unwrap();
        }
        total_length
    }
}

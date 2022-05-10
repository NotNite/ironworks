use std::{
	fmt::Display,
	io::{self, Read, Seek, SeekFrom},
};

use binrw::{binread, BinRead, VecArgs};
use flate2::read::DeflateDecoder;

use crate::error::{Error, Result};

const MAX_COMPRESSED_BLOCK_SIZE: u32 = 16_000;

pub fn read_file(mut reader: impl Read + Seek, offset: u32) -> Result<Vec<u8>> {
	// Move to the start of the file and read in the header.
	reader.seek(SeekFrom::Start(offset.into()))?;
	let header = Header::read(&mut reader)?;

	let expected_file_size = header.raw_file_size;

	// TODO: if type 1 and first 64 == second 64, RSF
	//       if type 1 and first 64 == [0..], empty

	let file_offset = offset + header.size;
	let out_buffer = match &header.kind {
		FileKind::Standard => read_standard(reader, file_offset, header),
		FileKind::Texture => read_texture(reader, file_offset, header),
		_ => todo!("File kind: {:?}", header.kind),
	}?;

	match out_buffer.len() == expected_file_size.try_into().unwrap() {
		true => Ok(out_buffer),
		false => Err(Error::Resource(
			read_failed("file", expected_file_size, out_buffer.len()).into(),
		)),
	}
}

fn read_standard(mut reader: impl Read + Seek, offset: u32, header: Header) -> Result<Vec<u8>> {
	// Eagerly read the block info.
	let blocks = <Vec<BlockInfo>>::read_args(
		&mut reader,
		VecArgs {
			count: header.block_count.try_into().unwrap(),
			inner: (),
		},
	)?;

	// Read each block into a final byte vector.
	let out_buffer = blocks.iter().try_fold(
		Vec::<u8>::with_capacity(header.raw_file_size.try_into().unwrap()),
		|mut vec, block_info| -> io::Result<Vec<u8>> {
			let mut block_reader = read_block(&mut reader, offset + block_info.offset)?;

			// Check we read the expected size.
			let count = block_reader.read_to_end(&mut vec)?;
			if count != block_info.decompressed_size.into() {
				return Err(io::Error::new(
					io::ErrorKind::Other,
					read_failed("block", block_info.decompressed_size, count),
				));
			}

			Ok(vec)
		},
	)?;

	Ok(out_buffer)
}

fn read_texture(mut reader: impl Read + Seek, offset: u32, header: Header) -> Result<Vec<u8>> {
	// Eagerly read the block info.
	let blocks = <Vec<LodBlockInfo>>::read_args(
		&mut reader,
		VecArgs {
			count: header.block_count.try_into().unwrap(),
			inner: (),
		},
	)?;

	// Directly after the block info, texture files have a table of sub-block offsets.
	let sub_block_count = blocks
		.iter()
		.fold(0, |total, block| total + block.block_count);

	let sub_block_offsets = <Vec<u16>>::read_args(
		&mut reader,
		VecArgs {
			count: sub_block_count.try_into().unwrap(),
			inner: (),
		},
	)?;

	// Create a vec with capacity for the full file.
	let mut out_basis = Vec::<u8>::with_capacity(header.raw_file_size.try_into().unwrap());

	// If the first block has an offset, it's likely that there's a .tex header
	// outside the compressed blocks - read the delta into the buffer as raw bytes.
	let raw_header_size = blocks[0].compressed_offset;
	if raw_header_size > 0 {
		reader.seek(SeekFrom::Start(offset.into()))?;
		reader
			.by_ref()
			.take(raw_header_size.into())
			.read_to_end(&mut out_basis)?;
	}

	let out_buffer = blocks
		.iter()
		// Scan the LOD sub block info to get the expected offsets.
		.flat_map(|lod_block| {
			let index_offset = usize::try_from(lod_block.block_offset).unwrap();
			(index_offset..usize::try_from(lod_block.block_count).unwrap() + index_offset).scan(
				lod_block.compressed_offset + offset,
				|next, index| {
					let offset = *next;
					*next += u32::from(sub_block_offsets[index]);
					Some(offset)
				},
			)
		})
		// Read the block data.
		.try_fold(out_basis, |mut vec, offset| -> io::Result<Vec<u8>> {
			let mut block_reader = read_block(&mut reader, offset)?;
			block_reader.read_to_end(&mut vec)?;
			Ok(vec)
		})?;

	Ok(out_buffer)
}

fn read_failed(item: impl Display, expected: impl Display, got: impl Display) -> String {
	format!("Failed to read {item}. Expected {expected} bytes, got {got}.",)
}

// TODO: move this into a block struct of some kind if we do lazy reading?
fn read_block<R: Read + Seek>(reader: &mut R, offset: u32) -> io::Result<BlockReader<R>> {
	// Seek to the block and read its header so we know how much to expect in the rest of the block.
	reader.seek(SeekFrom::Start(offset.into()))?;
	let block_header =
		BlockHeader::read(reader).map_err(|error| io::Error::new(io::ErrorKind::Other, error))?;

	// TODO: Look into the padding on compressed blocks, there's some funky stuff going on in some cases. Ref. Coinach/IO/File & Lumina.

	// Build a reader for the block.
	let reader = match block_header.compressed_size > MAX_COMPRESSED_BLOCK_SIZE {
		true => BlockReader::Loose(reader.take(block_header.decompressed_size.into())),
		false => BlockReader::Compressed(DeflateDecoder::new(
			reader.take(block_header.compressed_size.into()),
		)),
	};

	Ok(reader)
}

enum BlockReader<'a, R> {
	Loose(io::Take<&'a mut R>),
	Compressed(DeflateDecoder<io::Take<&'a mut R>>),
}

impl<R: Read> Read for BlockReader<'_, R> {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		match self {
			Self::Loose(reader) => reader.read(buf),
			Self::Compressed(reader) => reader.read(buf),
		}
	}
}

#[binread]
#[derive(Debug)]
#[br(little)]
struct Header {
	size: u32,
	kind: FileKind,
	raw_file_size: u32,
	// num_blocks: i32,
	// block_buffer_size: i32,
	#[br(pad_before = 8)]
	block_count: u32,
}

#[binread]
#[derive(Debug)]
#[br(little, repr = u32)]
enum FileKind {
	Empty = 1,
	Standard,
	Model,
	Texture,
}

#[binread]
#[derive(Debug)]
#[br(little)]
struct BlockInfo {
	offset: u32,
	_compressed_size: u16,
	decompressed_size: u16,
}

#[binread]
#[br(little)]
#[derive(Debug)]
struct LodBlockInfo {
	compressed_offset: u32,
	_compressed_size: u32,
	_decompressed_size: u32,
	block_offset: u32,
	block_count: u32,
}

#[binread]
#[derive(Debug)]
#[br(little)]
struct BlockHeader {
	_size: u32,
	// unknown1: u32,
	#[br(pad_before = 4)]
	compressed_size: u32,
	decompressed_size: u32,
}

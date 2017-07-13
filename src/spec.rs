use std::io;
use std::io::prelude::*;
use result::{ZipResult, ZipError};
use podio::{ReadPodExt, WritePodExt, LittleEndian};

pub const LOCAL_FILE_HEADER_SIGNATURE : u32 = 0x04034b50;
pub const CENTRAL_DIRECTORY_HEADER_SIGNATURE : u32 = 0x02014b50;
const CENTRAL_DIRECTORY_END_SIGNATURE : u32 = 0x06054b50;
const ZIP64_CENTRAL_DIRECTORY_END_SIGNATURE : u32 = 0x06064b50;
const ZIP64_CENTRAL_DIRECTORY_END_LOCATOR_SIGNATURE : u32 = 0x07064b50;

pub struct CentralDirectoryEnd
{
    pub disk_number: u16,
    pub disk_with_central_directory: u16,
    pub number_of_files_on_this_disk: u16,
    pub number_of_files: u16,
    pub central_directory_size: u32,
    pub central_directory_offset: u32,
    pub zip_file_comment: Vec<u8>,
}

impl CentralDirectoryEnd
{
    pub fn parse<T: Read>(reader: &mut T) -> ZipResult<CentralDirectoryEnd>
    {
        let magic = try!(reader.read_u32::<LittleEndian>());
        if magic != CENTRAL_DIRECTORY_END_SIGNATURE
        {
            return Err(ZipError::InvalidArchive("Invalid digital signature header"))
        }
        let disk_number = try!(reader.read_u16::<LittleEndian>());
        let disk_with_central_directory = try!(reader.read_u16::<LittleEndian>());
        let number_of_files_on_this_disk = try!(reader.read_u16::<LittleEndian>());
        let number_of_files = try!(reader.read_u16::<LittleEndian>());
        let central_directory_size = try!(reader.read_u32::<LittleEndian>());
        let central_directory_offset = try!(reader.read_u32::<LittleEndian>());
        let zip_file_comment_length = try!(reader.read_u16::<LittleEndian>()) as usize;
        let zip_file_comment = try!(ReadPodExt::read_exact(reader, zip_file_comment_length));

        Ok(CentralDirectoryEnd
           {
               disk_number: disk_number,
               disk_with_central_directory: disk_with_central_directory,
               number_of_files_on_this_disk: number_of_files_on_this_disk,
               number_of_files: number_of_files,
               central_directory_size: central_directory_size,
               central_directory_offset: central_directory_offset,
               zip_file_comment: zip_file_comment,
           })
    }

    pub fn find_and_parse<T: Read+io::Seek>(reader: &mut T) -> ZipResult<(CentralDirectoryEnd, u32)>
    {
        let header_size = 22;
        let bytes_between_magic_and_comment_size = header_size - 6;
        let file_length = try!(reader.seek(io::SeekFrom::End(0))) as i64;

        let search_upper_bound = ::std::cmp::max(0, file_length - header_size - ::std::u16::MAX as i64);

        let mut pos = file_length - header_size;
        while pos >= search_upper_bound
        {
            try!(reader.seek(io::SeekFrom::Start(pos as u64)));
            if try!(reader.read_u32::<LittleEndian>()) == CENTRAL_DIRECTORY_END_SIGNATURE
            {
                try!(reader.seek(io::SeekFrom::Current(bytes_between_magic_and_comment_size)));
                let comment_length = try!(reader.read_u16::<LittleEndian>()) as i64;
                if file_length - pos - header_size == comment_length
                {
                    let cde_start_pos = try!(reader.seek(io::SeekFrom::Start(pos as u64))) as u32;
                    return CentralDirectoryEnd::parse(reader).map(|cde| (cde, cde_start_pos));
                }
            }
            pos -= 1;
        }
        Err(ZipError::InvalidArchive("Could not find central directory end"))
    }

    pub fn write<T: Write>(&self, writer: &mut T) -> ZipResult<()>
    {
        try!(writer.write_u32::<LittleEndian>(CENTRAL_DIRECTORY_END_SIGNATURE));
        try!(writer.write_u16::<LittleEndian>(self.disk_number));

        try!(writer.write_u16::<LittleEndian>(self.disk_with_central_directory));
        try!(writer.write_u16::<LittleEndian>(self.number_of_files_on_this_disk));
        try!(writer.write_u16::<LittleEndian>(self.number_of_files));
        try!(writer.write_u32::<LittleEndian>(self.central_directory_size));
        try!(writer.write_u32::<LittleEndian>(self.central_directory_offset));
        try!(writer.write_u16::<LittleEndian>(self.zip_file_comment.len() as u16));
        try!(writer.write_all(&self.zip_file_comment));
        Ok(())
    }
}

pub struct Zip64CentralDirectoryEndLocator
{
    pub disk_with_central_directory: u32,
    pub end_of_central_directory_offset: u64,
    pub number_of_disks: u32,
}

impl Zip64CentralDirectoryEndLocator
{
    pub fn parse<T: Read>(reader: &mut T) -> ZipResult<Zip64CentralDirectoryEndLocator>
    {
        let magic = try!(reader.read_u32::<LittleEndian>());
        if magic != ZIP64_CENTRAL_DIRECTORY_END_LOCATOR_SIGNATURE
        {
            return Err(ZipError::InvalidArchive("Invalid zip64 locator digital signature header"))
        }
        let disk_with_central_directory = try!(reader.read_u32::<LittleEndian>());
        let end_of_central_directory_offset = try!(reader.read_u64::<LittleEndian>());
        let number_of_disks = try!(reader.read_u32::<LittleEndian>());

        Ok(Zip64CentralDirectoryEndLocator
           {
               disk_with_central_directory: disk_with_central_directory,
               end_of_central_directory_offset: end_of_central_directory_offset,
               number_of_disks: number_of_disks,
           })
    }
}

pub struct Zip64CentralDirectoryEnd
{
    pub version_made_by: u16,
    pub version_needed_to_extract: u16,
    pub disk_number: u32,
    pub disk_with_central_directory: u32,
    pub number_of_files_on_this_disk: u64,
    pub number_of_files: u64,
    pub central_directory_size: u64,
    pub central_directory_offset: u64,
    //pub extensible_data_sector: Vec<u8>,
}

impl Zip64CentralDirectoryEnd
{
    pub fn parse<T: Read>(reader: &mut T) -> ZipResult<Zip64CentralDirectoryEnd>
    {
        let magic = try!(reader.read_u32::<LittleEndian>());
        if magic != ZIP64_CENTRAL_DIRECTORY_END_SIGNATURE
        {
            return Err(ZipError::InvalidArchive("Invalid zip64 locator digital signature header"))
        }

        let _record_size = try!(reader.read_u64::<LittleEndian>());
        // XXX: we ignore the "zip64 extensible data sector" for now

        let version_made_by = try!(reader.read_u16::<LittleEndian>());
        let version_needed_to_extract = try!(reader.read_u16::<LittleEndian>());
        let disk_number = try!(reader.read_u32::<LittleEndian>());
        let disk_with_central_directory = try!(reader.read_u32::<LittleEndian>());
        let number_of_files_on_this_disk = try!(reader.read_u64::<LittleEndian>());
        let number_of_files = try!(reader.read_u64::<LittleEndian>());
        let central_directory_size = try!(reader.read_u64::<LittleEndian>());
        let central_directory_offset = try!(reader.read_u64::<LittleEndian>());

        Ok(Zip64CentralDirectoryEnd
           {
               version_made_by: version_made_by,
               version_needed_to_extract: version_needed_to_extract,
               disk_number: disk_number,
               disk_with_central_directory: disk_with_central_directory,
               number_of_files_on_this_disk: number_of_files_on_this_disk,
               number_of_files: number_of_files,
               central_directory_size: central_directory_size,
               central_directory_offset: central_directory_offset,
           })
    }
}

#![recursion_limit = "1024"]

#[macro_use]
extern crate cpp;

use std::{
    os::raw::c_char,
    ffi::CStr,
    io,
    path::Path,
};

#[cfg(windows)]
mod os
{
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    pub(crate) type NodSystemChar = u16;

    pub(crate) fn os_str_to_sys_char(s: &OsStr) -> Vec<NodSystemChar>
    {
        let mut v: Vec<_> = s.encode_wide().collect();
        v.push(0);
        v
    }
}

#[cfg(not(windows))]
mod os
{
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    pub(crate) type NodSystemChar = u8;

    pub(crate) fn os_str_to_sys_char(s: &OsStr) -> Vec<NodSystemChar>
    {
        let mut v: Vec<_> = s.as_bytes().to_owned();
        v.push(0);
        v
    }
}

use os::*;

cpp! {{
    #include <nod/nod.hpp>
    #include <nod/DiscBase.hpp>

    struct FileWrapper
    {
        std::shared_ptr<nod::DiscBase> disc;
        nod::Node &file;

        uint64_t read_bytes(uint64_t offset, uint64_t buf_length, uint8_t *buf)
        {
            try {
                auto stream = this->file.beginReadStream(offset);
                return stream->read(buf, buf_length);
            } catch (...) {
                return 0;
            }
        }

        FileWrapper(std::shared_ptr<nod::DiscBase> disc_, nod::Node &file_)
            : disc(std::move(disc_)), file(file_)
        { }
    };

    struct DiscWrapper
    {
        std::shared_ptr<nod::DiscBase> disc;

        static DiscWrapper* create(nod::SystemChar *disc_path, const char **err_msg)
        {
            try {
                bool is_wii;
                std::unique_ptr<nod::DiscBase> disc = nod::OpenDiscFromImage(disc_path, is_wii);
                if (!disc) {
                    *err_msg = "Failed to open disc";
                    return 0;
                }

                nod::IPartition* partition = disc->getDataPartition();
                if (!partition) {
                    *err_msg = "Failed to find data partition";
                    return 0;
                }

                return new DiscWrapper { std::shared_ptr<nod::DiscBase>(disc.release()) };
            } catch (...) {
                *err_msg = "Unknown error";
                return 0;
            }
        }

        FileWrapper* open_file(const char *file_name)
        {
            try {
                nod::IPartition* partition = this->disc->getDataPartition();
                if (!partition) {
                    return 0;
                }

                nod::Node &root = partition->getFSTRoot();
                nod::Node *found = nullptr;
                auto it_end = root.rawEnd();
                for(auto it = root.rawBegin(); it != it_end; ++it) {
                    if(it->getName() == file_name) {
                        found = &*it;
                        break;
                    }
                }

                if(!found) {
                    return 0;
                }

                return new FileWrapper(this->disc, *found);
            } catch (...) {
                return 0;
            }
        }
    };
}}

pub struct DiscWrapper(*const ());
impl DiscWrapper
{
    pub fn new<P>(disc_path: P) -> Result<DiscWrapper, String>
        where P: AsRef<Path>
    {
        let disc_path = os_str_to_sys_char(disc_path.as_ref().as_os_str());
        let disc_path = &disc_path[..] as *const [_] as *const NodSystemChar;

        let mut err_msg: *const c_char = std::ptr::null();
        let err_msg = &mut err_msg;

        let p = cpp!(unsafe [disc_path as "nod::SystemChar*", err_msg as "const char **"]
                            -> *const () as "DiscWrapper*" {
            return DiscWrapper::create(disc_path, err_msg);
        });

        if p.is_null() {
            Err(if !err_msg.is_null() {
                unsafe { CStr::from_ptr(*err_msg) }.to_string_lossy().into_owned()
            } else {
                "Unknown error".to_owned()
            })?
        }

        Ok(DiscWrapper(p))
    }

    pub fn open_file(&self, file_name: &CStr) -> Result<FileWrapper, String>
    {
        let self_ptr = self.0;
        let file_name_ptr = file_name.as_ptr();

        let p = cpp!(unsafe [self_ptr as "DiscWrapper*", file_name_ptr as "const char*"]
                            -> *const () as "FileWrapper*" {
            return self_ptr->open_file(file_name_ptr);
        });

        if p.is_null() {
            Err(format!("Failed to find file {}", &file_name.to_string_lossy()[..]))?
        }

        Ok(FileWrapper(p))
    }
}

impl Drop for DiscWrapper
{
    fn drop(&mut self)
    {
        let p = self.0;
        cpp!(unsafe [p as "DiscWrapper*"] {
            delete p;
        });
    }
}

#[derive(Debug)]
pub struct FileWrapper(*const ());
impl FileWrapper
{
    pub fn read_bytes(&self, offset: u64, buf: &mut [u8]) -> u64
    {
        let p = self.0;
        let buf_len = buf.len() as u64;
        let buf = buf as *mut [u8] as *mut u8;
        cpp!(unsafe [p as "FileWrapper*", offset as "uint64_t", buf_len as "uint64_t",
                     buf as "uint8_t*"]
                    -> u64 as "uint64_t" {
            return p->read_bytes(offset, buf_len, buf);
        })
    }

    pub fn len(&self) -> u64
    {
        let p = self.0;
        cpp!(unsafe [p as "FileWrapper*"] -> u64 as "uint64_t" {
            return p->file.size();
        })
    }
}

impl Drop for FileWrapper
{
    fn drop(&mut self)
    {
        let p = self.0;
        cpp!(unsafe [p as "FileWrapper*"] {
            delete p;
        });
    }
}

impl Clone for FileWrapper
{
    fn clone(&self) -> Self
    {
        let p = self.0;
        let p = cpp!(unsafe [p as "FileWrapper*"] -> *const () as "FileWrapper*" {
            return new FileWrapper(*p);
        });
        FileWrapper(p)
    }
}

impl reader_writer::WithRead for FileWrapper
{
    fn len(&self) -> usize
    {
        self.len() as usize
    }

    fn boxed<'a>(&self) -> Box<dyn reader_writer::WithRead + 'a>
        where Self: 'a
    {
        Box::new(self.clone())
    }

    fn with_read(&self, f: &mut dyn FnMut(&mut dyn io::Read) -> io::Result<u64>) -> io::Result<u64>
    {
        f(&mut FileWrapperRead {
            fw: self,
            offset: 0,
        })
    }
}

struct FileWrapperRead<'a>
{
    fw: &'a FileWrapper,
    offset: u64,
}

impl<'a> io::Read for FileWrapperRead<'a>
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>
    {
        let bytes_to_write = std::cmp::min(buf.len() as u64, self.fw.len() - self.offset) as usize;
        let i = self.fw.read_bytes(self.offset, &mut buf[..bytes_to_write]);
        self.offset += i;
        Ok(i as usize)
    }
}

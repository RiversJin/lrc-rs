use std::{pin::Pin, cell::UnsafeCell};

use liblrc_sys as ffi;


pub struct Lrc {
    lrc: ffi::lrc_t,
}
#[derive(Debug)]
pub enum LrcError {
    LrcOutOfMemory = -1,
    LrcUnrecoverable = -2,
    LrcInitTwice = -3,
    LrcInvalidArgument = -4,
    LrcIndexOverflow = -6,
    LrcBufOverflow = -7,
    LrcUnkonwn = -5,
}

impl Lrc {
    pub fn new(data_cnt: i32, group_cnt: i32, global_cnt: i32) -> Result<Self, LrcError> {
        let mut lrc = ffi::lrc_t::default();
        let mut local_arr = vec![(data_cnt / group_cnt) as u8; group_cnt as usize];
        match unsafe {ffi::lrc_init_n(&mut lrc, group_cnt, local_arr.as_mut_ptr(), global_cnt + group_cnt) } {
            -1 => Err(LrcError::LrcOutOfMemory),
            -2 => Err(LrcError::LrcUnrecoverable),
            -3 => Err(LrcError::LrcInitTwice),
            -4 => Err(LrcError::LrcInvalidArgument),
            i if i < 0 => Err(LrcError::LrcUnkonwn),
            _ => Ok(Self { lrc }),
        }
    }

    pub fn new_buf(&self, chunk_size: usize) -> Result<LrcBuf, LrcError> {
        let mut buf = Box::pin(ffi::lrc_buf_t::default());
        let mut lrc_cell = UnsafeCell::new(self.lrc);
        match unsafe {ffi::lrc_buf_init(&mut *buf, lrc_cell.get_mut(), chunk_size as i64)} {
            -1 => Err(LrcError::LrcOutOfMemory),
            -2 => Err(LrcError::LrcUnrecoverable),
            -3 => Err(LrcError::LrcInitTwice),
            -4 => Err(LrcError::LrcInvalidArgument),
            i if i < 0 => Err(LrcError::LrcUnkonwn),
            _ => Ok(LrcBuf { buf, lrc: self }),
        }
    }

    pub fn get_source(&self, erased: &Vec<i8>) -> Result<Vec<i8>, LrcError> {
        let mut lrc_cell = UnsafeCell::new(self.lrc);
        let mut source = vec![0i8; erased.len()];
        let erased = erased.as_ptr() as *mut i8;
        match unsafe {ffi::lrc_get_source(lrc_cell.get_mut(), erased, source.as_mut_ptr())} {
            -1 => Err(LrcError::LrcOutOfMemory),
            -2 => Err(LrcError::LrcUnrecoverable),
            -3 => Err(LrcError::LrcInitTwice),
            -4 => Err(LrcError::LrcInvalidArgument),
            i if i < 0 => Err(LrcError::LrcUnkonwn),
            _ => Ok(source),
        }
    }

}

impl Drop for Lrc {
    fn drop(&mut self) {
        unsafe { ffi::lrc_destroy(&mut self.lrc) };
    }
}

pub struct LrcBuf<'a>{
    buf: Pin<Box<ffi::lrc_buf_t>>,
    lrc: &'a Lrc,
}

impl<'a> LrcBuf<'a> {
    pub fn encode(&mut self) -> Result<(), LrcError> {
        let mut lrc_mut = UnsafeCell::new(self.lrc.lrc);
        match unsafe {ffi::lrc_encode(lrc_mut.get_mut(), &mut *self.buf)} {
            -1 => Err(LrcError::LrcOutOfMemory),
            -2 => Err(LrcError::LrcUnrecoverable),
            -3 => Err(LrcError::LrcInitTwice),
            -4 => Err(LrcError::LrcInvalidArgument),
            i if i < 0 => Err(LrcError::LrcUnkonwn),
            _ => Ok(()),
        }
    }

    pub fn decode(&mut self, erased: Vec<i8>) -> Result<(), LrcError> {
        let mut lrc_mut = UnsafeCell::new(self.lrc.lrc);
        match unsafe {ffi::lrc_decode(lrc_mut.get_mut(), &mut *self.buf, erased.as_ptr() as *mut i8)} {
            -1 => Err(LrcError::LrcOutOfMemory),
            -2 => Err(LrcError::LrcUnrecoverable),
            -3 => Err(LrcError::LrcInitTwice),
            -4 => Err(LrcError::LrcInvalidArgument),
            i if i < 0 => Err(LrcError::LrcUnkonwn),
            _ => Ok(()),
        }
    }

    pub fn set_data(&mut self, index: i32, buf: impl AsRef<[u8]>)->Result<(), LrcError> {
        if index < 0 || index >= self.lrc.lrc.k {
            return Err(LrcError::LrcIndexOverflow);
        }
        let buf = buf.as_ref();
        if buf.len() > self.buf.as_ref().chunk_size as usize {
            return Err(LrcError::LrcBufOverflow);
        }
        let target = self.buf.as_mut().data[index as usize];
        let target = unsafe {std::slice::from_raw_parts_mut(target as *mut u8, buf.len())};
        target.copy_from_slice(buf);
        Ok(())
    }

    pub fn get_code(&self, index: i32) -> Result<&[u8], LrcError> {
        if index < 0 || index >= self.lrc.lrc.m {
            return Err(LrcError::LrcIndexOverflow);
        }
        let target = unsafe{ *self.buf.code };
        let target = unsafe {std::slice::from_raw_parts(target as *mut u8, self.buf.as_ref().chunk_size as usize)};
        Ok(target)
    }
}

impl<'a> Drop for LrcBuf<'a> {
    fn drop(&mut self) {
        unsafe { ffi::lrc_buf_destroy(&mut *self.buf) };
    }
}
use std::io::Error;
use std::path::Path;
use crate::coroutine::YieldStatus;
use crate::fs::File;

#[inline(always)]
pub fn create_dir(path: Box<dyn AsRef<Path>>, res: *mut Result<(), Error>) -> YieldStatus {
    YieldStatus::create_dir(path, res)
}

#[inline(always)]
pub fn remove_dir(path: Box<dyn AsRef<Path>>, res: *mut Result<(), Error>) -> YieldStatus {
    YieldStatus::remove_dir(path, res)
}

#[inline(always)]
pub fn remove_file(path: Box<dyn AsRef<Path>>, res: *mut Result<(), Error>) -> YieldStatus {
    File::remove(path, res)
}
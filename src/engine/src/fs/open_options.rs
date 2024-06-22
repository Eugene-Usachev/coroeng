// TODO docs with examples
/// Options and flags which can be used to configure how a file is opened.
pub struct OpenOptions {
    /// This option, when true, will indicate that the file should be read-able if opened.
    pub read: bool,
    /// This option, when true, will indicate that the file should be write-able if opened.
    ///
    /// If the file already exists, any write calls on it will overwrite its contents, without truncating it.
    pub write: bool,
    /// This option, when true, means that writes will append to a file instead of overwriting previous contents.
    /// Note that setting [`.write(true)`](OpenOptions::write)[`.append(true)`](OpenOptions::append)
    /// has the same effect as setting only [`.append(true)`](OpenOptions::append).
    pub append: bool,
    /// If a file is successfully opened with this option set it will truncate the file to 0 length if it already exists.
    ///
    /// The file must be opened with write access for truncate to work.
    pub truncate: bool,
    /// This option, when true, will indicate that the file should be created if it does not exist. Else it will open the file.
    /// In order for the file to be created, OpenOptions::write or OpenOptions::append access must be used.
    pub create: bool,
    /// This option, when true, will indicate that the file should be created if it does not exist.
    /// Else it will return an [`error`](std::io::ErrorKind::AlreadyExists).
    ///
    /// For more information, see [`OpenOptions::create_new`](OpenOptions::create_new).
    pub create_new: bool,
    /// Pass custom flags to the flags argument of open.
    ///
    /// For more information, see [`OpenOptions::custom_flags`](OpenOptions::custom_flags).
    pub custom_flags: u32,
}

impl OpenOptions {
    /// Creates a blank new set of options ready for configuration.
    ///
    /// All options are initially set to false.
    pub fn new() -> Self {
        Self {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
            custom_flags: 0
        }
    }

    /// Sets the option for read access.
    ///
    /// This option, when true, will indicate that the file should be read-able if opened.
    pub fn read(mut self, read: bool) -> Self {
        self.read = read;
        self
    }

    /// Sets the option for write access.
    ///
    /// This option, when true, will indicate that the file should be write-able if opened.
    ///
    /// If the file already exists, any write calls on it will overwrite its contents, without truncating it.
    pub fn write(mut self, write: bool) -> Self {
        self.write = write;
        self
    }

    /// Sets the option for the append mode.
    ///
    /// This option, when true, means that writes will append to a file instead of overwriting previous contents.
    /// Note that setting [`.write(true)`](OpenOptions::write)[`.append(true)`](OpenOptions::append)
    /// has the same effect as setting only [`.append(true)`](OpenOptions::append).
    pub fn append(mut self, append: bool) -> Self {
        self.append = append;
        self
    }

    /// Sets the option for truncating a previous file.
    ///
    /// If a file is successfully opened with this option set it will truncate the file to 0 length if it already exists.
    ///
    /// The file must be opened with write access for truncate to work.
    pub fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }

    /// Sets the option to create a new file, or open it if it already exists.
    /// In order for the file to be created, [`OpenOptions::write`] or [`OpenOptions::append`] access must be used.
    pub fn create(mut self, create: bool) -> Self {
        self.create = create;
        self
    }

    /// Sets the option to create a new file, failing if it already exists.
    ///
    /// No file is allowed to exist at the target location, also no (dangling) symlink. In this way, if the call succeeds,
    /// the file returned is guaranteed to be new. If a file exists at the target location, creating a new file will fail
    /// with [`AlreadyExists`](std::io::ErrorKind::AlreadyExists) or another error based on the situation. See OpenOptions::open for a non-exhaustive list of likely errors.
    ///
    /// This option is useful because it is atomic. Otherwise, between checking whether a file exists and creating a new one,
    /// the file may have been created by another process (a TOCTOU race condition / attack).
    ///
    /// If [`.create_new(true)`](OpenOptions::create_new) is set, [`.create()`](OpenOptions::create)
    /// and [`.truncate()`](OpenOptions::truncate) are ignored.
    ///
    /// The file must be opened with write or append access in order to create a new file.
    pub fn create_new(mut self, create_new: bool) -> Self {
        self.create_new = create_new;
        self
    }


    /// Pass custom flags to the flags argument of open.
    ///
    /// The bits that define the access mode are masked out with O_ACCMODE,
    /// to ensure they do not interfere with the access mode set by Rusts options.
    ///
    /// Custom flags can only set flags, not remove flags set by Rusts options. This options overwrites any previously set custom flags.
    pub fn custom_flags(mut self, flags: u32) -> Self {
        self.custom_flags = flags;
        self
    }
}
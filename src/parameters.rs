// Copyright (c) 2018-2023, agnos.ai UK Ltd, all rights reserved.
//---------------------------------------------------------------

extern crate alloc;

use {
    crate::{
        database_call,
        root::{
            CParameters,
            CParameters_destroy,
            CParameters_newEmptyParameters,
            CParameters_setString,
        },
    },
    alloc::ffi::CString,
    rdf_store_rs::{consts::LOG_TARGET_DATABASE, RDFStoreError},
    std::{
        fmt::{Display, Formatter},
        path::Path,
        ptr,
    },
};

pub enum FactDomain {
    ASSERTED,
    INFERRED,
    ALL,
}

pub enum PersistenceMode {
    File,
    FileSequence,
    Off,
}

impl Display for PersistenceMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistenceMode::File => write!(f, "file"),
            PersistenceMode::FileSequence => write!(f, "file-sequence"),
            PersistenceMode::Off => write!(f, "off"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameters {
    pub(crate) inner: *mut CParameters,
}

unsafe impl Sync for Parameters {}

unsafe impl Send for Parameters {}

impl Display for Parameters {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parameters[]") // TODO: show keys and values (currently not
        // possible)
    }
}

impl Drop for Parameters {
    fn drop(&mut self) {
        assert!(
            !self.inner.is_null(),
            "Parameters-object was already dropped"
        );
        unsafe {
            CParameters_destroy(self.inner);
            tracing::trace!(target: LOG_TARGET_DATABASE, "Destroyed params");
        }
    }
}

impl Parameters {
    pub fn empty() -> Result<Self, RDFStoreError> {
        let mut parameters: *mut CParameters = ptr::null_mut();
        database_call!(
            "Allocating parameters",
            CParameters_newEmptyParameters(&mut parameters)
        )?;
        Ok(Parameters { inner: parameters })
    }

    pub fn set_string(&self, key: &str, value: &str) -> Result<(), RDFStoreError> {
        let c_key = CString::new(key).unwrap();
        let c_value = CString::new(value).unwrap();
        let msg = format!("Setting parameter {c_key:?}={c_value:?}");
        database_call!(
            msg.as_str(),
            CParameters_setString(self.inner, c_key.as_ptr(), c_value.as_ptr())
        )
    }

    pub fn fact_domain(self, fact_domain: FactDomain) -> Result<Self, RDFStoreError> {
        match fact_domain {
            FactDomain::ASSERTED => self.set_string("fact-domain", "explicit")?,
            FactDomain::INFERRED => self.set_string("fact-domain", "derived")?,
            FactDomain::ALL => self.set_string("fact-domain", "all")?,
        };
        Ok(self)
    }

    pub fn switch_off_file_access_sandboxing(self) -> Result<Self, RDFStoreError> {
        self.set_string("sandbox-directory", "")?;
        Ok(self)
    }

    pub fn persist_datastore(self, mode: PersistenceMode) -> Result<Self, RDFStoreError> {
        self.set_string("persist-ds", &mode.to_string())?;
        Ok(self)
    }

    pub fn persist_roles(self, mode: PersistenceMode) -> Result<Self, RDFStoreError> {
        self.set_string("persist-roles", &mode.to_string())?;
        Ok(self)
    }

    pub fn server_directory(self, dir: &Path) -> Result<Self, RDFStoreError> {
        if dir.is_dir() {
            self.set_string("server-directory", dir.to_str().unwrap())?;
            Ok(self)
        } else {
            panic!("{dir:?} is not a directory")
        }
    }

    pub fn license_file(self, file: &Path) -> Result<Self, RDFStoreError> {
        if file.is_file() {
            self.set_string("license-file", file.to_str().unwrap())?;
            Ok(self)
        } else {
            panic!("{file:?} does not exist")
        }
    }

    pub fn import_rename_user_blank_nodes(self, setting: bool) -> Result<Self, RDFStoreError> {
        self.set_string(
            "import.rename-user-blank-nodes",
            format!("{setting:?}").as_str(),
        )?;
        Ok(self)
    }

    /// If true, all API calls are recorded in a script that
    /// the shell can replay later. later.
    /// The default value is false.
    pub fn api_log(self, on: bool) -> Result<Self, RDFStoreError> {
        if on {
            self.set_string("api-log", "on")?;
        } else {
            self.set_string("api-log", "off")?;
        }
        Ok(self)
    }

    /// Specifies the directory into which API logs will be written.
    /// Default is directory api-log within the configured server directory.
    pub fn api_log_directory(self, dir: &Path) -> Result<Self, RDFStoreError> {
        self.set_string("api-log.directory", dir.to_str().unwrap())?;
        Ok(self)
    }
}

// Copyright (c) 2018-2022, agnos.ai UK Ltd, all rights reserved.
//---------------------------------------------------------------

extern crate alloc;

use alloc::ffi::CString;
use std::{
    fmt::{Display, Formatter},
    path::Path,
    ptr,
};

use crate::{
    database_call,
    error::Error,
    root::{
        CParameters,
        CParameters_destroy,
        CParameters_newEmptyParameters,
        CParameters_setString,
    },
};

pub enum FactDomain {
    ASSERTED,
    INFERRED,
    ALL,
}

pub struct Parameters {
    pub(crate) inner: *mut CParameters,
}

impl Display for Parameters {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parameters[]") // TODO: show keys and values (currently not
                                  // possible)
    }
}

impl Drop for Parameters {
    fn drop(&mut self) {
        unsafe {
            CParameters_destroy(self.inner);
            log::trace!("Destroyed params");
        }
    }
}

impl Parameters {
    pub fn empty() -> Result<Self, Error> {
        let mut parameters: *mut CParameters = ptr::null_mut();
        database_call!(
            "allocating parameters",
            CParameters_newEmptyParameters(&mut parameters)
        )?;
        Ok(Parameters {
            inner: parameters
        })
    }

    pub fn set_string(&self, key: &str, value: &str) -> Result<(), Error> {
        let c_key = CString::new(key).unwrap();
        let c_value = CString::new(value).unwrap();
        database_call!(
            "setting a parameter",
            CParameters_setString(self.inner, c_key.as_ptr(), c_value.as_ptr())
        )?;
        log::debug!("param {key}={value}");
        Ok(())
    }

    pub fn fact_domain(self, fact_domain: FactDomain) -> Result<Self, Error> {
        match fact_domain {
            FactDomain::ASSERTED => self.set_string("fact-domain", "explicit")?,
            FactDomain::INFERRED => self.set_string("fact-domain", "derived")?,
            FactDomain::ALL => self.set_string("fact-domain", "all")?,
        };
        Ok(self)
    }

    pub fn switch_off_file_access_sandboxing(self) -> Result<Self, Error> {
        self.set_string("sandbox-directory", "")?;
        Ok(self)
    }

    /// If true, all API calls are recorded in a script that
    /// the shell can replay later. later.
    /// The default value is false.
    pub fn api_log(self, on: bool) -> Result<Self, Error> {
        if on {
            self.set_string("api-log", "on")?;
        } else {
            self.set_string("api-log", "off")?;
        }
        Ok(self)
    }

    /// Specifies the directory into which API logs will be written.
    /// Default is directory api-log within the configured server directory.
    pub fn api_log_directory(self, dir: &Path) -> Result<Self, Error> {
        self.set_string("api-log.directory", dir.to_str().unwrap())?;
        Ok(self)
    }
}

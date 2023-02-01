// Copyright (c) 2018-2023, agnos.ai UK Ltd, all rights reserved.
//---------------------------------------------------------------

use {
    crate::{
        Cursor,
        database_call,
        RDFStoreError::{self, Unknown},
        root::{
            CArgumentIndex,
            CCursor,
            CCursor_advance,
            CCursor_getAnswerVariableName,
            CCursor_getArgumentIndexes,
            CCursor_getArgumentsBuffer,
            CCursor_getArity,
            CCursor_open,
            CResourceID,
        },
        Transaction,
    },
    rdf_store_rs::RDFStoreError::CannotGetAnyArgumentIndexes,
    std::{ptr, sync::Arc},
};

#[derive(Debug)]
pub struct OpenedCursor<'a> {
    pub tx: Arc<Transaction>,
    pub cursor: &'a Cursor,
    /// the arity (i.e., the number of columns) of the answers that the
    /// cursor computes.
    pub arity: usize,
    pub arguments_buffer: &'a [u64],
    pub argument_indexes: &'a [u32],
}

impl<'a> OpenedCursor<'a> {
    /// Open the cursor, get the details like arity and argument info and
    /// return it as a tuple with all the details (except multiplicity)
    /// as an `OpenedCursor` and the multiplicity of the first row.
    pub(crate) fn new(
        cursor: &'a mut Cursor,
        tx: Arc<Transaction>,
    ) -> Result<(Self, u64), RDFStoreError> {
        let c_cursor = cursor.inner;
        let multiplicity = Self::open(cursor.inner)?;
        let arity = Self::arity(c_cursor)?;
        let arguments_buffer = Self::arguments_buffer(c_cursor)?;
        let argument_indexes = Self::argument_indexes(cursor, c_cursor, arity)?;
        let opened_cursor = OpenedCursor {
            tx,
            cursor,
            arity,
            arguments_buffer,
            argument_indexes,
        };
        Ok((opened_cursor, multiplicity))
    }

    fn open(c_cursor: *mut CCursor) -> Result<u64, RDFStoreError> {
        let mut multiplicity = 0 as usize;
        database_call!(
            "opening a cursor",
            CCursor_open(c_cursor, &mut multiplicity)
        )?;
        tracing::debug!("CCursor_open ok multiplicity={multiplicity}");
        Ok(multiplicity as u64)
    }

    /// Returns the arity (i.e., the number of columns) of the answers that the
    /// cursor computes.
    fn arity(c_cursor: *mut CCursor) -> Result<usize, RDFStoreError> {
        let mut arity = 0_usize;
        database_call!(
            "getting the arity",
            CCursor_getArity(c_cursor, &mut arity)
        )?;
        Ok(arity)
    }

    pub fn arguments_buffer(c_cursor: *mut CCursor) -> Result<&'a [u64], RDFStoreError> {
        let mut buffer: *const CResourceID = ptr::null_mut();
        database_call!(
            "getting the arguments buffer",
            CCursor_getArgumentsBuffer(c_cursor, &mut buffer)
        )?;
        let mut count = 0_usize;
        unsafe {
            let mut p = buffer;
            while !p.is_null() {
                count += 1;
                let resource_id: CResourceID = *p as CResourceID;
                if resource_id == 0 {
                    break;
                }
                tracing::trace!("{count} resource_id={:?}", resource_id);
                p = p.offset(1);
            }
        }
        unsafe { Ok(std::slice::from_raw_parts(buffer, count - 1)) }
    }

    fn argument_indexes(
        cursor: &Cursor,
        c_cursor: *mut CCursor,
        arity: usize,
    ) -> Result<&'a [u32], RDFStoreError> {
        let mut indexes: *const CArgumentIndex = ptr::null_mut();
        database_call!(
            "getting the argument-indexes",
            CCursor_getArgumentIndexes(c_cursor, &mut indexes)
        )?;
        if indexes.is_null() {
            return Err(CannotGetAnyArgumentIndexes { query: cursor.sparql_string().to_string() });
        }
        unsafe {
            Ok(std::slice::from_raw_parts(
                indexes,
                arity as usize,
            ))
        }
    }

    /// Get the resource ID from the arguments buffer which dynamically changes
    /// after each cursor advance.
    pub(crate) fn resource_id(&self, term_index: usize) -> Result<Option<u64>, RDFStoreError> {
        if let Some(argument_index) = self.argument_indexes.get(term_index as usize) {
            if let Some(resource_id) = self.arguments_buffer.get(*argument_index as usize) {
                Ok(Some(*resource_id))
            } else {
                tracing::error!(
                    "Could not get the resource ID from the arguments buffer with argument \
                    index {argument_index} and term index {term_index}"
                );
                // Err(Unknown)
                Ok(None)
            }
        } else {
            tracing::error!("Could not get the argument index for term index {term_index}");
            Err(Unknown)
        }
    }

    /// TODO: Check why this panics when called after previous call returned
    /// zero
    pub fn advance(&mut self) -> Result<u64, RDFStoreError> {
        let mut multiplicity = 0_usize;
        database_call!(
            "advancing the cursor",
            CCursor_advance(self.cursor.inner, &mut multiplicity)
        )?;
        tracing::trace!(
            "cursor {:?} advanced, multiplicity={multiplicity}",
            self.cursor.inner
        );
        Ok(multiplicity as u64)
    }

    pub fn update_and_commit<T, U>(&mut self, f: T) -> Result<U, RDFStoreError>
        where T: FnOnce(&mut OpenedCursor) -> Result<U, RDFStoreError> {
        Transaction::begin_read_write(&self.cursor.connection)?.update_and_commit(|_tx| f(self))
    }

    pub fn execute_and_rollback<T, U>(&mut self, f: T) -> Result<U, RDFStoreError>
        where T: FnOnce(&mut OpenedCursor) -> Result<U, RDFStoreError> {
        Transaction::begin_read_only(&self.cursor.connection)?.execute_and_rollback(|_tx| f(self))
    }

    /// Get the variable name used in the executed SPARQL statement representing
    /// the given column in the output.
    ///
    /// ```rust
    /// use rdfox::root;
    /// extern "C" {
    ///     pub fn CCursor_getAnswerVariableName(
    ///         cursor: *mut root::CCursor,
    ///         variable_index: usize,
    ///         answer_variable_name: *mut *const std::os::raw::c_char,
    ///     ) -> *const root::CException;
    /// }
    /// ```
    pub fn get_answer_variable_name(&self, index: usize) -> Result<String, RDFStoreError> {
        let mut c_buf: *const std::os::raw::c_char = ptr::null();
        database_call!(
            "getting a variable name",
            CCursor_getAnswerVariableName(self.cursor.inner, index as usize, &mut c_buf)
        )?;
        let c_name = unsafe { std::ffi::CStr::from_ptr(c_buf) };
        Ok(c_name.to_str().unwrap().to_owned())
    }
}

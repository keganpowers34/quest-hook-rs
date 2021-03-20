use std::borrow::Cow;
use std::fmt;
use std::mem::transmute;
use std::{ffi::CStr, slice};

use crate::raw::{
    self, METHOD_ATTRIBUTE_ABSTRACT, METHOD_ATTRIBUTE_STATIC, METHOD_ATTRIBUTE_VIRTUAL,
};
use crate::{
    Arguments, Il2CppClass, Il2CppException, Il2CppObject, Il2CppType, ParameterInfo, Return, This,
    WrapRaw,
};

/// Information about a C# method
#[repr(transparent)]
pub struct MethodInfo(raw::MethodInfo);

impl MethodInfo {
    /// Invoke this method, type checking against its signature with the
    /// provided instance, arguments and return type
    pub fn invoke<T, A, R, const N: usize>(&self, this: T, args: A) -> Result<R, &Il2CppException>
    where
        T: This,
        A: Arguments<N>,
        R: Return,
    {
        assert!(T::matches(self));
        assert!(A::matches(self.parameters()));
        assert!(R::matches(self.return_ty()));

        unsafe { self.invoke_unchecked(this, args) }
    }

    /// Invoke this method with the given instance and arguments and converting
    /// the result to the specified type, without type checking
    ///
    /// # Safety
    /// To be safe, the provided types have to match the method signature
    pub unsafe fn invoke_unchecked<T, A, R, const N: usize>(
        &self,
        this: T,
        args: A,
    ) -> Result<R, &Il2CppException>
    where
        T: This,
        A: Arguments<N>,
        R: Return,
    {
        let mut exception = None;
        let r = raw::runtime_invoke(
            self.raw(),
            this.invokable(),
            args.invokable().as_mut_ptr(),
            &mut exception,
        );
        let r = transmute::<Option<&mut raw::Il2CppObject>, Option<&mut Il2CppObject>>(r);
        match exception {
            None => Ok(R::from_object(r)),
            Some(e) => Err(Il2CppException::wrap(e)),
        }
    }

    /// Name of the method
    pub fn name(&self) -> Cow<'_, str> {
        let name = self.raw().name;
        assert!(!name.is_null());
        unsafe { CStr::from_ptr(name) }.to_string_lossy()
    }

    /// Class the method is from
    pub fn class(&self) -> &Il2CppClass {
        unsafe { Il2CppClass::wrap_ptr(self.raw().klass) }.unwrap()
    }

    /// Return type of the method
    pub fn return_ty(&self) -> &Il2CppType {
        unsafe { Il2CppType::wrap_ptr(self.raw().return_type).unwrap() }
    }

    /// Parameters the method takes
    pub fn parameters(&self) -> &[&ParameterInfo] {
        let parameters = self.raw().parameters;
        if !parameters.is_null() {
            unsafe { slice::from_raw_parts(parameters as _, self.raw().parameters_count as _) }
        } else {
            &[]
        }
    }

    /// Whether the method is static
    pub fn is_static(&self) -> bool {
        self.raw().flags as u32 & METHOD_ATTRIBUTE_STATIC != 0
    }

    /// Whether the method is abstract
    pub fn is_abstract(&self) -> bool {
        self.raw().flags as u32 & METHOD_ATTRIBUTE_ABSTRACT != 0
    }

    /// Whether the method is virtual
    pub fn is_virtual(&self) -> bool {
        self.raw().flags as u32 & METHOD_ATTRIBUTE_VIRTUAL != 0
    }
}

unsafe impl WrapRaw for MethodInfo {
    type Raw = raw::MethodInfo;
}

impl fmt::Debug for MethodInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MethodInfo")
            .field("class", self.class())
            .field("name", &self.name())
            .field("parameters", &self.parameters())
            .field("static", &self.is_static())
            .field("abstract", &self.is_abstract())
            .field("virtual", &self.is_virtual())
            .finish()
    }
}

impl fmt::Display for MethodInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params = self.parameters();
        let n = params.len() - 1;

        if self.is_static() {
            f.write_str("static ")?;
        }
        if self.is_abstract() {
            f.write_str("abstract ")?;
        }
        if self.is_virtual() {
            f.write_str("virtual ")?;
        }

        write!(f, "{} {}(", self.return_ty(), self.name())?;
        for p in &params[..n] {
            write!(f, "{}, ", p)?;
        }
        write!(f, "{})", params[n])
    }
}

// Copyright (c) 2018 Manuel Reinhardt
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

//! Contains the `DrawFuncs` trait.

use crate::bindings::{
    hb_draw_funcs_create, hb_draw_funcs_destroy, hb_draw_funcs_get_empty, hb_draw_funcs_reference,
    hb_draw_funcs_set_close_path_func, hb_draw_funcs_set_cubic_to_func,
    hb_draw_funcs_set_line_to_func, hb_draw_funcs_set_move_to_func,
    hb_draw_funcs_set_quadratic_to_func, hb_draw_funcs_t, hb_draw_state_t,
};
use crate::common::{HarfbuzzObject, Owned, Shared};
use crate::font::destroy_box;

use std::os::raw::c_void;

use std::{self, fmt, marker::PhantomData, panic, ptr::NonNull};

#[derive(Copy, Clone, Debug)]
pub struct DrawState {
    pub path_open: bool,
    pub path_start_x: f32,
    pub path_start_y: f32,
    pub current_x: f32,
    pub current_y: f32,
}

/// This Trait specifies the font callbacks that harfbuzz uses when asked
/// to draw a glyph.
#[allow(unused_variables)]
pub trait DrawFuncs {
    fn move_to(&mut self, st: &DrawState, to_x: f32, to_y: f32);
    fn line_to(&mut self, st: &DrawState, to_x: f32, to_y: f32);
    fn quadratic_to(
        &mut self,
        st: &DrawState,
        control_x: f32,
        control_y: f32,
        to_x: f32,
        to_y: f32,
    );
    #[allow(clippy::too_many_arguments)]
    fn cubic_to(
        &mut self,
        st: &DrawState,
        control1_x: f32,
        control1_y: f32,
        control2_x: f32,
        control2_y: f32,
        to_x: f32,
        to_y: f32,
    );
    fn close_path(&mut self, st: &DrawState);
}

macro_rules! hb_callback {
    ($func_name:ident<$($arg:ident: $datatype:ty),*>{
        $(argument $closure_arg:ty => $expr:expr,)*
    }) => {
        #[allow(clippy::let_and_return)]
        extern "C" fn $func_name<T, F>(
            _dfuncs: *mut hb_draw_funcs_t,
            draw_data: *mut ::std::os::raw::c_void,
            $(
                $arg: $datatype,
            )*
            closure_data: *mut c_void,
        ) where F: Fn(&mut T, $($closure_arg),*) {
            let catch_result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                let draw_data = unsafe { &mut *(draw_data as *mut T) };
                let closure = unsafe { &mut *(closure_data as *mut F) };
                closure(draw_data, $($expr),*);
            }));
            match catch_result {
                Ok(val) => val,
                Err(_) => {
                    // TODO: Log error
                    Default::default()
                }
            }
        }
    };
}

hb_callback!(
    rust_move_to_closure<st: *mut hb_draw_state_t, to_x: f32, to_y: f32> {
        argument DrawState => DrawState {
            path_open: unsafe { (*st).path_open != 0 },
            path_start_x: unsafe { (*st).path_start_x },
            path_start_y: unsafe { (*st).path_start_y },
            current_x: unsafe { (*st).current_x },
            current_y: unsafe { (*st).current_y }
        },
        argument f32 => to_x,
        argument f32 => to_y,
    }
);

hb_callback!(
    rust_line_to_closure <st: *mut hb_draw_state_t, to_x: f32, to_y: f32> {
        argument DrawState => DrawState {
            path_open: unsafe { (*st).path_open != 0 },
            path_start_x: unsafe { (*st).path_start_x },
            path_start_y: unsafe { (*st).path_start_y },
            current_x: unsafe { (*st).current_x },
            current_y: unsafe { (*st).current_y }
        },
        argument f32 => to_x,
        argument f32 => to_y,
    }
);

hb_callback!(
    rust_quadratic_to_closure <st: *mut hb_draw_state_t, control_x: f32, control_y: f32, to_x: f32, to_y: f32> {
        argument DrawState => DrawState {
            path_open: unsafe { (*st).path_open != 0 },
            path_start_x: unsafe { (*st).path_start_x },
            path_start_y: unsafe { (*st).path_start_y },
            current_x: unsafe { (*st).current_x },
            current_y: unsafe { (*st).current_y }
        },
        argument f32 => control_x,
        argument f32 => control_y,
        argument f32 => to_x,
        argument f32 => to_y,
    }
);

hb_callback!(
    rust_cubic_to_closure <st: *mut hb_draw_state_t, control1_x: f32, control1_y: f32,  control2_x: f32, control2_y: f32, to_x: f32, to_y: f32> {
        argument DrawState => DrawState {
            path_open: unsafe { (*st).path_open != 0 },
            path_start_x: unsafe { (*st).path_start_x },
            path_start_y: unsafe { (*st).path_start_y },
            current_x: unsafe { (*st).current_x },
            current_y: unsafe { (*st).current_y }
        },
        argument f32 => control1_x,
        argument f32 => control1_y,
        argument f32 => control2_x,
        argument f32 => control2_y,
        argument f32 => to_x,
        argument f32 => to_y,
    }
);
hb_callback!(
    rust_close_path_closure <st: *mut hb_draw_state_t> {
        argument DrawState => DrawState {
            path_open: unsafe { (*st).path_open != 0 },
            path_start_x: unsafe { (*st).path_start_x },
            path_start_y: unsafe { (*st).path_start_y },
            current_x: unsafe { (*st).current_x },
            current_y: unsafe { (*st).current_y }
        },
    }
);

/// A `DrawFuncsImpl` contains implementations of the font callbacks that
/// harfbuzz uses to draw a glyph.
///
/// To use this, set the font funcs from a type that implements the `DrawFuncs`
/// trait using the `from_trait_impl` constructor.
///
/// # Example
///
/// ```ignore
/// use harfbuzz_rs::*;
/// use harfbuzz_rs::draw_funcs::DrawFuncsImpl;
///
/// // Dummy struct implementing DrawFuncs
/// struct MyDrawFuncs {
///    points: Vec<(f32,f32)>,
/// }
/// impl DrawFuncs for MyDrawFuncs {
///     fn move_to(&self, _: &DrawState, x: f32, y: f32) {
///         // ...
///     }
///     // implementations of other functions...
/// }
///
/// let draw_funcs: Owned<DrawFuncsImpl<MyFontData>> = DrawFuncsImpl::from_trait_impl();
/// ```
pub(crate) struct DrawFuncsImpl<T> {
    raw: NonNull<hb_draw_funcs_t>,
    marker: PhantomData<T>,
}

impl<T> DrawFuncsImpl<T> {
    /// Returns an empty `DrawFuncsImpl`. Every font callback of the returned
    /// `DrawFuncsImpl` gives a null value regardless of its input.
    #[allow(unused)]
    pub fn empty() -> Shared<DrawFuncsImpl<T>> {
        let raw = unsafe { hb_draw_funcs_get_empty() };
        unsafe { Shared::from_raw_ref(raw) }
    }
}

impl<T: DrawFuncs> DrawFuncsImpl<T> {
    /// Create a new `DrawFuncsImpl` from the `DrawFuncs` trait implementation
    /// of `T`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use harfbuzz_rs::*;
    /// use harfbuzz_rs::draw_funcs::DrawFuncsImpl;
    ///
    /// // Dummy struct implementing DrawFuncs
    /// struct MyDrawFuncs {
    ///    points: Vec<(f32,f32)>,
    /// }
    /// impl DrawFuncs for MyDrawFuncs {
    ///     fn move_to(&self, _: &DrawState, x: f32, y: f32) {
    ///         // ...
    ///     }
    ///     // implementations of other functions...
    /// }
    ///
    /// let draw_funcs: Owned<DrawFuncsImpl<MyFontData>> = DrawFuncsImpl::from_trait_impl();
    /// ```
    ///
    pub fn from_trait_impl() -> Owned<DrawFuncsImpl<T>> {
        let mut ffuncs = DrawFuncsImpl::new();
        ffuncs.set_trait_impl();
        ffuncs
    }

    fn set_trait_impl(&mut self) {
        self.set_move_to_func(|data, st, x, y| data.move_to(&st, x, y));
        self.set_line_to_func(|data, st, x, y| data.line_to(&st, x, y));
        self.set_quadratic_to_func(|data, st, cx, cy, x, y| data.quadratic_to(&st, cx, cy, x, y));
        self.set_cubic_to_func(|data, st, c1x, c1y, c2x, c2y, x, y| {
            data.cubic_to(&st, c1x, c1y, c2x, c2y, x, y)
        });
        self.set_close_path_func(|data, st| data.close_path(&st));
    }
}

impl<T> DrawFuncsImpl<T> {
    pub fn new() -> Owned<DrawFuncsImpl<T>> {
        unsafe { Owned::from_raw(hb_draw_funcs_create()) }
    }

    pub fn set_move_to_func<F>(&mut self, func: F)
    where
        F: Fn(&mut T, DrawState, f32, f32),
    {
        let user_data = Box::new(func);
        unsafe {
            hb_draw_funcs_set_move_to_func(
                self.as_raw(),
                Some(rust_move_to_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_quadratic_to_func<F>(&mut self, func: F)
    where
        F: Fn(&mut T, DrawState, f32, f32, f32, f32),
    {
        let user_data = Box::new(func);
        unsafe {
            hb_draw_funcs_set_quadratic_to_func(
                self.as_raw(),
                Some(rust_quadratic_to_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_line_to_func<F>(&mut self, func: F)
    where
        F: Fn(&mut T, DrawState, f32, f32),
    {
        let user_data = Box::new(func);
        unsafe {
            hb_draw_funcs_set_line_to_func(
                self.as_raw(),
                Some(rust_line_to_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_cubic_to_func<F>(&mut self, func: F)
    where
        F: Fn(&mut T, DrawState, f32, f32, f32, f32, f32, f32),
    {
        let user_data = Box::new(func);
        unsafe {
            hb_draw_funcs_set_cubic_to_func(
                self.as_raw(),
                Some(rust_cubic_to_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }

    pub fn set_close_path_func<F>(&mut self, func: F)
    where
        F: Fn(&mut T, DrawState),
    {
        let user_data = Box::new(func);
        unsafe {
            hb_draw_funcs_set_close_path_func(
                self.as_raw(),
                Some(rust_close_path_closure::<T, F>),
                Box::into_raw(user_data) as *mut _,
                Some(destroy_box::<F>),
            );
        }
    }
}

impl<T> fmt::Debug for DrawFuncsImpl<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DrawFuncsImpl")
            .field("raw", &self.as_raw())
            .finish()
    }
}

unsafe impl<T> HarfbuzzObject for DrawFuncsImpl<T> {
    type Raw = hb_draw_funcs_t;

    unsafe fn from_raw(raw: *const Self::Raw) -> Self {
        DrawFuncsImpl {
            raw: NonNull::new(raw as *mut _).unwrap(),
            marker: PhantomData,
        }
    }

    fn as_raw(&self) -> *mut Self::Raw {
        self.raw.as_ptr()
    }

    unsafe fn reference(&self) {
        hb_draw_funcs_reference(self.as_raw());
    }

    unsafe fn dereference(&self) {
        hb_draw_funcs_destroy(self.as_raw())
    }
}

unsafe impl<T> Send for DrawFuncsImpl<T> {}
unsafe impl<T> Sync for DrawFuncsImpl<T> {}

#[cfg(test)]
mod tests {
    use crate::draw_funcs::DrawFuncs;

    use crate::{Face, Font, *};
    use std::path::PathBuf;

    #[repr(C)]
    #[derive(Debug)]
    struct TestDrawFuncs {
        output: String,
    }
    impl DrawFuncs for TestDrawFuncs {
        fn move_to(&mut self, _st: &draw_funcs::DrawState, to_x: f32, to_y: f32) {
            self.output.push_str(&format!("M {:} {:} ", to_x, to_y));
        }

        fn line_to(&mut self, _st: &draw_funcs::DrawState, to_x: f32, to_y: f32) {
            self.output.push_str(&format!("L {:} {:} ", to_x, to_y));
        }

        fn quadratic_to(
            &mut self,
            _st: &draw_funcs::DrawState,
            control_x: f32,
            control_y: f32,
            to_x: f32,
            to_y: f32,
        ) {
            self.output.push_str(&format!(
                "Q {:} {:}, {:} {:} ",
                control_x, control_y, to_x, to_y
            ));
        }

        fn cubic_to(
            &mut self,
            _st: &draw_funcs::DrawState,
            control1_x: f32,
            control1_y: f32,
            control2_x: f32,
            control2_y: f32,
            to_x: f32,
            to_y: f32,
        ) {
            self.output.push_str(&format!(
                "C {:} {:}, {:}, {:}, {:} {:} ",
                control1_x, control1_y, control2_x, control2_y, to_x, to_y
            ));
        }

        fn close_path(&mut self, _st: &draw_funcs::DrawState) {
            self.output.push('Z');
        }
    }

    #[test]
    fn test_move_to() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("testfiles/SourceSansVariable-Roman.ttf");
        let face = Face::from_file(path, 0).expect("Error reading font file.");
        let font = Font::new(face);
        let shape = TestDrawFuncs {
            output: String::new(),
        };
        font.draw_glyph(2, &shape);
        println!("After");
        assert_eq!(shape.output, "M 10 0 L 246 660 L 274 660 L 510 0 L 476 0 L 338 396 Q 317 456, 298.5 510 Q 280 564, 262 626 L 258 626 Q 240 564, 221.5 510 Q 203 456, 182 396 L 42 0 L 10 0 ZM 112 236 L 112 264 L 405 264 L 405 236 L 112 236 Z");
    }
}

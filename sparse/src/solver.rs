use std::{fmt::Display, ptr};

use sprs::CsMatView;
use suitesparse_sys::{
    SPQR_DEFAULT_TOL, SPQR_ORDERING_DEFAULT, SPQR_QX, SPQR_RTX_EQUALS_ETB,
    SuiteSparseQR_C_factorization, SuiteSparseQR_C_factorize, SuiteSparseQR_C_free,
    SuiteSparseQR_C_qmult, SuiteSparseQR_C_solve, cholmod_common, cholmod_l_allocate_dense,
    cholmod_l_allocate_sparse, cholmod_l_finish, cholmod_l_free_dense, cholmod_l_free_sparse,
    cholmod_l_start, cholmod_l_transpose,
};

#[derive(Debug)]
pub enum Error {
    AllocationFailed,
    FactorizationFailed,
    SolveFailed,
    QmultFailed,
    NotFactorized,
    DimensionMismatch,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AllocationFailed => write!(f, "failed to allocate memory"),
            Self::FactorizationFailed => write!(f, "QR factorization faield"),
            Self::SolveFailed => write!(f, "failed to solve linear system"),
            Self::QmultFailed => write!(f, "failed to apply Q matrix"),
            Self::NotFactorized => write!(f, "Solver not factorized"),
            Self::DimensionMismatch => write!(f, "Matrix/vector dimension mismatch"),
        }
    }
}

impl std::error::Error for Error {}

pub struct MinNormSolver {
    common: cholmod_common,
    qr_factorization: Option<*mut SuiteSparseQR_C_factorization>,
    matrix_shape: Option<(usize, usize)>,
}

impl MinNormSolver {
    /// # Errors
    /// If memory allocation failed
    pub fn new() -> Result<Self, Error> {
        let mut common = unsafe { std::mem::zeroed::<cholmod_common>() };
        if unsafe { cholmod_l_start(&raw mut common) } == 0 {
            return Err(Error::AllocationFailed);
        }
        Ok(Self {
            common,
            qr_factorization: None,
            matrix_shape: None,
        })
    }

    /// # Errors
    /// - `Error::AllocationFailed` if memory allocation fails
    /// # Panics
    /// - If it fails to get the index pointer from the matrix
    pub fn factorize(&mut self, matrix: CsMatView<f64>) -> Result<(), Error> {
        let m = matrix.rows();
        let n = matrix.cols();
        let nnz = matrix.nnz();

        let mut a =
            unsafe { cholmod_l_allocate_sparse(m, n, nnz, 1, 1, 0, 1, &raw mut self.common) };

        if a.is_null() {
            return Err(Error::AllocationFailed);
        }

        unsafe {
            let a_ref = &mut *a;
            let p = a_ref.p.cast::<i64>();
            let i = a_ref.i.cast::<i64>();
            let x = a_ref.x.cast::<f64>();

            if !p.is_null() && !i.is_null() && !x.is_null() {
                let csc_matrix = matrix.to_csc();
                let indptr_ref = csc_matrix.indptr();
                let indptr = indptr_ref.as_slice().unwrap();
                let indices = csc_matrix.indices();
                let data = csc_matrix.data();

                for (idx, &item) in indptr.iter().enumerate() {
                    *p.add(idx) = i64::try_from(item).expect("non-wrapping value");
                }
                for (idx, &item) in indices.iter().enumerate() {
                    *i.add(idx) = i64::try_from(item).expect("non-wrapping value");
                }
                for (idx, &item) in data.iter().enumerate() {
                    *x.add(idx) = item;
                }
            }
        }

        let mut a_transpose = unsafe { cholmod_l_transpose(a, 1, &raw mut self.common) };
        if a_transpose.is_null() {
            unsafe { cholmod_l_free_sparse((&raw mut a).cast(), &raw mut self.common) };
            return Err(Error::AllocationFailed);
        }

        let qr = unsafe {
            SuiteSparseQR_C_factorize(
                SPQR_ORDERING_DEFAULT.cast_signed(),
                f64::from(SPQR_DEFAULT_TOL),
                a_transpose,
                &raw mut self.common,
            )
        };
        unsafe {
            cholmod_l_free_sparse((&raw mut a_transpose).cast(), &raw mut self.common);
            cholmod_l_free_sparse((&raw mut a).cast(), &raw mut self.common);
        }
        if qr.is_null() {
            return Err(Error::FactorizationFailed);
        }

        if let Some(old_qr) = self.qr_factorization {
            unsafe {
                SuiteSparseQR_C_free(&mut (old_qr.cast()), &raw mut self.common);
            };
        }

        self.qr_factorization = Some(qr);
        self.matrix_shape = Some((m, n));

        Ok(())
    }

    /// # Errors
    /// - `Error::NotFactorized` if QR factorization fails
    /// - `Error::DimensionMismatch` if `b` is not the same as `m`
    /// - `Error::AllocationFailed` if memory fails to be allocated
    /// - `Error::SolveFailed` if the solver fails to find a solution
    /// - `Error::QmultFailed` if the solver fails to generate the solution
    pub fn solve(&mut self, b: &[f64]) -> Result<Vec<f64>, Error> {
        let qr = self.qr_factorization.ok_or(Error::NotFactorized)?;
        let (m, _) = self.matrix_shape.ok_or(Error::NotFactorized)?;

        if b.len() != m {
            return Err(Error::DimensionMismatch);
        }

        let mut b_dense = unsafe { cholmod_l_allocate_dense(m, 1, m, 1, &raw mut self.common) };

        if b_dense.is_null() {
            return Err(Error::AllocationFailed);
        }

        unsafe {
            let b_ref = &mut *b_dense;
            let x = b_ref.x.cast::<f64>();
            ptr::copy_nonoverlapping(b.as_ptr(), x, b.len());
        }

        let mut y = unsafe {
            SuiteSparseQR_C_solve(
                SPQR_RTX_EQUALS_ETB.cast_signed(),
                qr,
                b_dense,
                &raw mut self.common,
            )
        };

        if y.is_null() {
            unsafe { cholmod_l_free_dense((&raw mut b_dense).cast(), &raw mut self.common) };
            return Err(Error::SolveFailed);
        }

        let mut x =
            unsafe { SuiteSparseQR_C_qmult(SPQR_QX.cast_signed(), qr, y, &raw mut self.common) };

        if x.is_null() {
            unsafe {
                cholmod_l_free_dense((&raw mut y).cast(), &raw mut self.common);
                cholmod_l_free_dense((&raw mut b_dense).cast(), &raw mut self.common);
            }
            return Err(Error::QmultFailed);
        }

        let result = unsafe {
            let x_ref = &*x;
            let x_ptr = x_ref.x as *const f64;
            let len = x_ref.nrow * x_ref.ncol;
            let mut vec = vec![0.0; len];
            ptr::copy_nonoverlapping(x_ptr, vec.as_mut_ptr(), len);
            vec
        };

        unsafe {
            cholmod_l_free_dense((&raw mut x).cast(), &raw mut self.common);
            cholmod_l_free_dense((&raw mut y).cast(), &raw mut self.common);
            cholmod_l_free_dense((&raw mut b_dense).cast(), &raw mut self.common);
        }

        Ok(result)
    }

    /// # Errors
    /// If the factorization or solver fails
    pub fn solve_min_norm(&mut self, matrix: CsMatView<f64>, b: &[f64]) -> Result<Vec<f64>, Error> {
        self.factorize(matrix)?;
        self.solve(b)
    }

    /// # Errors
    /// If the solver fails
    pub fn solve_multiple(&mut self, bs: &[&[f64]]) -> Result<Vec<Vec<f64>>, Error> {
        bs.iter().map(|b| self.solve(b)).collect()
    }
}

impl Drop for MinNormSolver {
    fn drop(&mut self) {
        unsafe {
            if let Some(qr) = self.qr_factorization {
                SuiteSparseQR_C_free(&mut (qr.cast()), &raw mut self.common);
            }
            cholmod_l_finish(&raw mut self.common);
        }
    }
}

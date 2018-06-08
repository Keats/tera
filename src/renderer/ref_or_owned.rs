//! Models a value that contains either a reference or an owned instance

// --- module use statements ---

use std::ops::Deref;

// --- module enum definitions ---

/// Models a value that contains either a reference or an owned instance.
///
/// For performance, best if sizeof T vs sizeof a ref is not too large
///
#[derive(Clone, Debug, Serialize)]
pub enum RefOrOwned<'a, T>
where
  T: 'a,
{
  /// The reference variant
  Borrowed {
    /// The instance borrowed
    borrow: &'a T,
  },
  /// The owned variant
  Owned {
    /// The owned instance
    owned: T,
  },
}

// --- module impl definitions ---

/// Implementation for type `RefOrOwned`.
impl<'a, T> RefOrOwned<'a, T>
where
  T: Clone,
{
  /// Get reference to value
  ///
  ///  * _return_ - Reference to the value
  ///
  #[inline]
  pub fn get(&self) -> &T {
    match self {
      RefOrOwned::Borrowed { borrow } => *borrow,
      RefOrOwned::Owned { owned } => owned,
    }
  }

  /// Get reference to value with extended lifetime.
  ///
  /// Requires stored value to be borrowed. See also `is_borrowed`.
  ///
  ///  * _return_ - Reference to the value
  ///
  #[inline]
  pub fn get_ref(&self) -> Option<&'a T> {
    match self {
      RefOrOwned::Borrowed { borrow } => Some(borrow),
      _ => None,
    }
  }

  /// Determine if stored value is borrowed
  ///
  ///  * _return_ - Return true of stored value is borrowed
  ///
  #[inline]
  pub fn is_borrowed(&self) -> bool {
    match self {
      RefOrOwned::Borrowed { borrow } => true,
      _ => false,
    }
  }

  /// Determine if stored value is owned
  ///
  ///  * _return_ - Return true of stored value is owned
  ///
  #[inline]
  pub fn is_owned(&self) -> bool {
    match self {
      RefOrOwned::Owned { owned } => true,
      _ => false,
    }
  }

  /// Take reference to value
  ///
  ///  * _return_ - Reference to the value
  ///
  #[inline]
  pub fn take(&mut self) -> T
  where
    T: Default,
  {
    match self {
      RefOrOwned::Borrowed { borrow } => (*borrow).clone(),
      RefOrOwned::Owned { owned } => ::std::mem::replace(owned, T::default()),
    }
  }

  /// Create from borrow
  ///
  ///  * `borrow` - Borrowed value
  ///  * _return_ - New wrapper with reference to value
  ///
  #[inline]
  pub fn from_borrow(borrow: &'a T) -> RefOrOwned<'a, T> {
    RefOrOwned::Borrowed { borrow }
  }

  /// Create from owned
  ///
  ///  * `owned` - Owned value
  ///  * _return_ - New wrapper with owned
  ///
  #[inline]
  pub fn from_owned(owned: T) -> RefOrOwned<'a, T> {
    RefOrOwned::Owned { owned }
  }
}

/// Implementation of trait `Deref` for type `RefOrOwned<'a, T>`
impl<'a, T> Deref for RefOrOwned<'a, T>
where
  T: Clone,
{
  type Target = T;

  /// Method to dereference a value.
  ///
  ///  * _return_ - The dereferenced value
  ///
  #[inline]
  fn deref(&self) -> &Self::Target {
    self.get()
  }
}

/// Test module for ref_or_owned module
#[cfg(test)]
mod tests {
  use super::*;
  mod ref_or_owned {

    use super::*;

    #[test]
    fn from_borrow() -> () {
      #[derive(Clone, Default)]
      struct T {
        t: String,
      }

      let t = T { t: "some t".into() };
      let ref_or_owned = RefOrOwned::from_borrow(&t);
      assert_eq!(ref_or_owned.get().t, t.t);
    }

    #[test]
    fn from_owned() -> () {
      #[derive(Clone, Default, PartialEq, Debug)]
      struct T {
        t: String,
      }

      let t = T { t: "some t".into() };
      let mut ref_or_owned = RefOrOwned::from_owned(t.clone());
      assert_eq!(ref_or_owned.get().t, t.t);
      let taken = ref_or_owned.take();
      assert_eq!(taken, t);
      assert_eq!(ref_or_owned.take(), T::default());
    }
  }
}

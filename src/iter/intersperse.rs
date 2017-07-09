use super::internal::*;
use super::*;


/// `Intersperse` is a parallel iterator adapter that inserts clones of an item
/// between the elements of a base iterator.
///
/// This struct is created by the [`intersperse()`] method on [`ParallelIterator`].
/// See its documentation for more information.
///
/// [`intersperse()`]: trait.ParallelIterator.html#method.intersperse
/// [`ParallelIterator`]: trait.ParallelIterator.html
#[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
pub struct Intersperse<I> 
    where I: ParallelIterator,
          I::Item: Send + Clone
{
    base: I,
    interspersed: I::Item,
}

/// Create a new `Intersperse` iterator.
///
/// NB: a free fn because it is NOT part of the end-user API.
pub fn new<I>(base: I, interspersed: I::Item) -> Intersperse<I>
    where I: ParallelIterator,
          I::Item: Send + Clone
{
    Intersperse { base: base, interspersed: interspersed }
}

impl<I> ParallelIterator for Intersperse<I>
    where I: ParallelIterator,
          I::Item: Send + Clone
{
    type Item = I::Item;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where C: UnindexedConsumer<Self::Item>
    {
        let consumer = IntersperseConsumer {
            base: consumer,
            interspersed: self.interspersed,
            next: IntersperseNext::IteratorItem
        };
        self.base.drive_unindexed(consumer)
    }
}

// ////////////////////////////////////////////////////////////////////////
// Consumer implementation

struct IntersperseConsumer<C, It> {
    base: C,
    interspersed: It,
    next: IntersperseNext
}

impl<C, It> IntersperseConsumer<C, It>
    where It: Clone
{
    fn new(base: C, interspersed: &It, next: IntersperseNext) -> Self {
        IntersperseConsumer {
            base: base,
            interspersed: interspersed.clone(),
            next: next
        }
    }
}

impl<C, It> Consumer<It> for IntersperseConsumer<C, It>
    where C: Consumer<It>,
          It: Send + Clone
{
    type Folder = IntersperseFolder<C::Folder, It>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
       let (left, right, reducer) = self.base.split_at(index);
       (IntersperseConsumer::new(left, &self.interspersed, self.next),
        IntersperseConsumer::new(right, &self.interspersed, IntersperseNext::Interspersed),
        reducer)
    }

    fn into_folder(self) -> Self::Folder {
        IntersperseFolder {
            base: self.base.into_folder(),
            interspersed: self.interspersed,
            next: self.next,
        }
    }

    fn full(&self) -> bool {
        self.base.full()
    }
}

// TODO Implement unindexed consumer for IntersperseConsumer
impl<C, It> UnindexedConsumer<It> for IntersperseConsumer<C, It>
    where C: UnindexedConsumer<It>,
          It: Send + Clone
{
    fn split_off_left(&self) -> Self {
        unimplemented!()
    }

    fn to_reducer(&self) -> Self::Reducer {
        unimplemented!()
    }
}

#[derive(Clone,Copy,PartialEq,Eq)]
enum IntersperseNext {
    Interspersed,
    IteratorItem,
}

struct IntersperseFolder<C, It> {
    base: C,
    interspersed: It,
    next: IntersperseNext
}

impl<C, It> Folder<It> for IntersperseFolder<C, It>
    where C: Folder<It>,
          It: Send + Clone
{
    type Result = C::Result;
    fn consume(self, item: It) -> Self {
        let ret = if self.next == IntersperseNext::Interspersed {
            IntersperseFolder {
                base: self.base.consume(self.interspersed.clone()),
                interspersed: self.interspersed,
                next: IntersperseNext::IteratorItem,
            }
        } else {
            self
        };
        IntersperseFolder {
            base: ret.base.consume(item),
            interspersed: ret.interspersed,
            next: IntersperseNext::Interspersed,
        }
    }

    fn complete(self) -> C::Result {
        self.base.complete()
    }

    fn full(&self) -> bool {
        self.base.full()
    }
}

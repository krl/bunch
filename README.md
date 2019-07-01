# Bunch

An append-only, concurrent arena.

## Guarantees

Elements pushed into the arena are never moved or modified. All the elements are dropped at once, as the arena is dropped.

Since the elements does not move in memory, they can be safely concurrently accessed by multiple threads.

Locking is only neccesary when pushing to the structure, as to guarantee that the allocation logic in different threads are not stepping on each others toes.

## Drawbacks

The elements are allocated in multiple slices on the heap, so the range prefix is not supported. Iterators could be supported though.

## Memory layout

The slices are arranged in the following pattern in memory:

```
[T, T]
[T, T, T, T]
[T, T, T, T, T, T, T, T]
```

Each new allocation is double the size of the previous one.

In order to efficiently calculate the row and column from an index, we can use a special instruction `usize::leading_zeros()`, which acts kind of as a log2 operation.

```rust
fn lane_offset(offset: usize) -> (usize, usize) {
    let i = offset / 2 + 1;
    let lane = USIZE_BITS - i.leading_zeros() as usize - 1;
    let offset = offset - (2usize.pow(lane as u32) - 1) * 2;
    (lane, offset)
}
```


## Entry

The entry is the main data structure.
It is a path to an image, along with its associated tags.

### Fields

```
struct Entry {
    /// Unique id by which this entry can always be uniquely identified
    id: u32,
    /// Path of the image by which it can be opened.
    path: PathBuffer,
    /// The tags associated with this entry.
    /// Tags are also identified by unique ids.
    tags: Vec<TagId>
}
```

## Tag

A tag can be used to mark an entry with an identifiable quality by which it can be searched for.
Since there can be multiple synonyms for the same concept, the same tag can have multiple names.

### Fields

```
struct Tag {
    /// Unique id
    id: u32,
    /// Names for this tag. e.g. `happy, merry, cheerful`.
    names: Vec<String>,
    /// Tags that this tag implies. e.g. `elephant` implies `pachyderm` and `animal`.
    implies: Vec<u32>,
}
```

# HashMap vs. Vec for storing id-referred items.

Vec would be obviously faster and more compact, and removal of items would leave no gaps.
However, since closing the gap means shifting all items to cover the gap, all referrers that refer
to ids greater than the removed item's id would get invalidated. They would have to be manually updated to make sure it points to the new, correct ids. This is more complicated to implement than just using a HashMap.

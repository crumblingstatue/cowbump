use {
    crate::{
        collection::{Sequences, Tags},
        db::{TagSet, Uid},
        dlog,
        filter_reqs::{Req, Requirements},
        tag,
    },
    serde_derive::{Deserialize, Serialize},
    std::path::PathBuf,
};

/// Path to an item we're interested in organizing, along with associated tags
#[derive(Serialize, Deserialize)]
pub struct Entry {
    /// Image path relative to collection root. Assumed to be unique within the collection.
    pub path: PathBuf,
    pub tags: TagSet,
}

#[derive(Hash, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Id(pub Uid);

impl Entry {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            tags: Default::default(),
        }
    }
    pub fn all_reqs_satisfied(
        &self,
        id: Id,
        reqs: &Requirements,
        tags: &Tags,
        sequences: &Sequences,
    ) -> bool {
        reqs.all(|req| self.req_satisfied(id, req, tags, sequences))
    }
    pub fn req_satisfied(&self, id: Id, req: &Req, tags: &Tags, sequences: &Sequences) -> bool {
        match req {
            Req::Any(reqs) => reqs.any(|req| self.req_satisfied(id, req, tags, sequences)),
            Req::All(reqs) => reqs.all(|req| self.req_satisfied(id, req, tags, sequences)),
            Req::None(reqs) => reqs.none(|req| self.req_satisfied(id, req, tags, sequences)),
            Req::Tag(id) => self.satisfies_required_tag(*id, tags),
            Req::Not(req) => !self.req_satisfied(id, req, tags, sequences),
            Req::FilenameSub(fsub) => self.path.to_string_lossy().to_lowercase().contains(fsub),
            Req::PartOfSeq => sequences.values().any(|seq| seq.contains_entry(id)),
            Req::Untagged => self.tags.is_empty(),
        }
    }
    fn satisfies_required_tag(&self, required_tag_id: tag::Id, tags: &Tags) -> bool {
        self.tags
            .iter()
            .any(|tag_id| tag_satisfies_required_tag(*tag_id, required_tag_id, tags, &mut 0))
    }
}

fn tag_satisfies_required_tag(
    tag_id: tag::Id,
    required_tag_id: tag::Id,
    tags: &Tags,
    depth: &mut u32,
) -> bool {
    *depth += 1;
    if *depth == 10 {
        dlog!(
            "Tag satisfies depth limit exceeded. Aborting [tag: {:?}, required: {:?}]",
            tags.get(&tag_id).and_then(|tag| tag.names.first()),
            tags.get(&required_tag_id).and_then(|tag| tag.names.first()),
        );
        return false;
    }
    if tag_id == required_tag_id {
        return true;
    }
    // Check if any implied tags satisfy required
    let tag = &tags[&tag_id];
    tag.implies.iter().any(|implied_tag_id| {
        tag_satisfies_required_tag(*implied_tag_id, required_tag_id, tags, depth)
    })
}

pub fn filter_map(
    uid: Id,
    entry: &Entry,
    reqs: &Requirements,
    tags: &Tags,
    sequences: &Sequences,
) -> Option<Id> {
    if entry.all_reqs_satisfied(uid, reqs, tags, sequences) {
        Some(uid)
    } else {
        None
    }
}

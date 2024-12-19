use {
    crate::{
        collection::{Collection, Tags, TagsExt},
        tag,
    },
    std::borrow::Cow,
    tagfilter_lang::Requirement,
    thiserror::Error,
};

#[derive(Default, Debug, PartialEq)]
pub struct Requirements {
    reqs: Vec<Req>,
}

impl Requirements {
    pub fn parse_and_resolve<'src>(
        &mut self,
        text: &'src str,
        coll: &Collection,
    ) -> Result<(), ParseResolveError<'src>> {
        let requirements = tagfilter_lang::parse(text)?;
        self.resolve(requirements, coll)?;
        Ok(())
    }
    pub fn resolve<'src>(
        &mut self,
        requirements: Vec<Requirement<'src>>,
        coll: &Collection,
    ) -> Result<(), ReqTransformError<'src>> {
        self.reqs.clear();
        for requirement in requirements {
            self.reqs
                .push(Req::from_tagfilter_lang_req(requirement, coll)?);
        }
        Ok(())
    }
    pub fn clear(&mut self) {
        self.reqs.clear();
    }
    pub fn have_tag(&self, id: tag::Id) -> bool {
        self.any(|req| req == &Req::Tag(id))
    }
    pub fn have_tag_exact(&self, id: tag::Id) -> bool {
        self.any(|req| req == &Req::TagExact(id))
    }
    /// It's required that the item doesn't have this tag
    pub fn not_have_tag(&self, id: tag::Id) -> bool {
        self.any(|req| req == &Req::Not(Box::new(Req::Tag(id))))
    }
    /// Note that this is top-level only. It might result in conflicting requirements.
    pub fn toggle_have_tag(&mut self, id: tag::Id) {
        self.set_have_tag(id, !self.have_tag(id));
    }
    /// Note that this is top-level only. It might result in conflicting requirements.
    pub fn toggle_have_tag_exact(&mut self, id: tag::Id) {
        self.set_have_tag_exact(id, !self.have_tag_exact(id));
    }
    /// Note that this is top-level only. It might result in conflicting requirements.
    fn add_tag(&mut self, id: tag::Id) {
        self.reqs.push(Req::Tag(id));
    }
    /// Note that this is top-level only. It might result in conflicting requirements.
    fn add_tag_exact(&mut self, id: tag::Id) {
        self.reqs.push(Req::TagExact(id));
    }
    /// Note that this is top-level only. It might result in conflicting requirements.
    fn remove_tag(&mut self, id: tag::Id) {
        self.reqs.retain(|req| req != &Req::Tag(id));
    }
    /// Note that this is top-level only. It might result in conflicting requirements.
    fn remove_tag_exact(&mut self, id: tag::Id) {
        self.reqs.retain(|req| req != &Req::TagExact(id));
    }
    /// Note that this is top-level only. It might result in conflicting requirements.
    fn add_not_tag(&mut self, id: tag::Id) {
        self.reqs.push(Req::Not(Box::new(Req::Tag(id))));
    }
    /// Note that this is top-level only. It might result in conflicting requirements.
    fn remove_not_tag(&mut self, id: tag::Id) {
        self.reqs
            .retain(|req| req != &Req::Not(Box::new(Req::Tag(id))));
    }
    /// Note that this is top-level only. It might result in conflicting requirements.
    pub fn set_have_tag(&mut self, id: tag::Id, have: bool) {
        if have {
            self.add_tag(id);
        } else {
            self.remove_tag(id);
        }
    }
    /// Note that this is top-level only. It might result in conflicting requirements.
    pub fn set_have_tag_exact(&mut self, id: tag::Id, have: bool) {
        if have {
            self.add_tag_exact(id);
        } else {
            self.remove_tag_exact(id);
        }
    }
    /// Note that this is top-level only. It might result in conflicting requirements.
    pub fn toggle_not_have_tag(&mut self, id: tag::Id) {
        self.set_not_have_tag(id, !self.not_have_tag(id));
    }
    /// Note that this is top-level only. It might result in conflicting requirements.
    pub fn set_not_have_tag(&mut self, id: tag::Id, not_have: bool) {
        if not_have {
            self.add_not_tag(id);
        } else {
            self.remove_not_tag(id);
        }
    }
    pub fn to_string(&self, tags: &Tags) -> String {
        let mut buf = String::new();
        for req in &self.reqs {
            buf += &req.to_string(tags);
            buf += " ";
        }
        buf
    }
    pub fn is_empty(&self) -> bool {
        self.reqs.is_empty()
    }
    pub fn any(&self, f: impl FnMut(&Req) -> bool) -> bool {
        self.reqs.iter().any(f)
    }
    pub fn all(&self, f: impl FnMut(&Req) -> bool) -> bool {
        self.reqs.iter().all(f)
    }
    pub fn none(&self, f: impl FnMut(&Req) -> bool) -> bool {
        !self.any(f)
    }
    /// Only considers top level
    pub(crate) fn have_tag_by_name(&self, name: &str, coll: &Collection) -> bool {
        match coll.resolve_tag(name) {
            Some(id) => self.have_tag(id),
            None => false,
        }
    }
    /// Only considers top level
    pub(crate) fn not_have_tag_by_name(&self, name: &str, coll: &Collection) -> bool {
        match coll.resolve_tag(name) {
            Some(id) => self.not_have_tag(id),
            None => false,
        }
    }
}

/// Cowbump specific requirements, transformed from `tagfilter_lang::Requirement`
#[derive(Debug, PartialEq)]
pub enum Req {
    Any(Requirements),
    All(Requirements),
    None(Requirements),
    Tag(tag::Id),
    // TODO: Implement in tagfilter_lang
    TagExact(tag::Id),
    Not(Box<Req>),
    FilenameSub(String),
    PartOfSeq,
    NTags(usize),
}

#[derive(Debug, Error)]
pub enum ReqTransformError<'src> {
    #[error("Unknown function: {name}")]
    UnknownFn { name: &'src str },
    #[error("No such tag: {0}")]
    NoSuchTag(&'src str),
    #[error("Missing parameter")]
    MissingParameter,
    #[error("Invalid parameter")]
    InvalidParameter,
}

impl Req {
    fn from_tagfilter_lang_req<'src>(
        tf_req: Requirement<'src>,
        coll: &Collection,
    ) -> Result<Self, ReqTransformError<'src>> {
        let req = match tf_req {
            Requirement::Tag(name) => {
                let id = coll
                    .resolve_tag(name)
                    .ok_or(ReqTransformError::NoSuchTag(name))?;
                Req::Tag(id)
            }
            Requirement::TagExact(name) => {
                let id = coll
                    .resolve_tag(name)
                    .ok_or(ReqTransformError::NoSuchTag(name))?;
                Req::TagExact(id)
            }
            Requirement::FnCall(call) => match call.name {
                "any" => {
                    let mut reqs = Requirements::default();
                    reqs.resolve(call.params, coll)?;
                    Req::Any(reqs)
                }
                "all" => {
                    let mut reqs = Requirements::default();
                    reqs.resolve(call.params, coll)?;
                    Req::All(reqs)
                }
                "none" => {
                    let mut reqs = Requirements::default();
                    reqs.resolve(call.params, coll)?;
                    Req::None(reqs)
                }
                "filename" | "file" | "fname" | "f" => {
                    let filename_sub = match call.params.first() {
                        Some(param) => match param {
                            Requirement::Tag(tag) | Requirement::TagExact(tag) => tag,
                            Requirement::FnCall(_) | Requirement::Not(_) => {
                                return Err(ReqTransformError::InvalidParameter);
                            }
                        },
                        None => return Err(ReqTransformError::MissingParameter),
                    };
                    Req::FilenameSub((*filename_sub).to_owned())
                }
                "seq" | "sequence" => Req::PartOfSeq,
                "notag" | "no-tag" | "untagged" => Req::NTags(0),
                "ntags" => match call.params.first() {
                    Some(Requirement::Tag(tag) | Requirement::TagExact(tag)) => {
                        match tag.parse::<usize>() {
                            Ok(n) => Req::NTags(n),
                            Err(_) => return Err(ReqTransformError::InvalidParameter),
                        }
                    }
                    Some(_) => return Err(ReqTransformError::InvalidParameter),
                    None => return Err(ReqTransformError::MissingParameter),
                },
                _ => return Err(ReqTransformError::UnknownFn { name: call.name }),
            },
            Requirement::Not(req) => Req::Not(Box::new(Req::from_tagfilter_lang_req(*req, coll)?)),
        };
        Ok(req)
    }

    fn to_string<'a>(&self, tags: &'a Tags) -> Cow<'a, str> {
        match self {
            Req::Any(reqs) => format!("@any[{}]", reqs.to_string(tags)).into(),
            Req::All(reqs) => format!("@all[{}]", reqs.to_string(tags)).into(),
            Req::None(reqs) => format!("@none[{}]", reqs.to_string(tags)).into(),
            Req::Tag(id) => tags.first_name_of(id),
            Req::TagExact(id) => ["$", &tags.first_name_of(id)].concat().into(),
            Req::Not(req) => format!("!{}", req.to_string(tags)).into(),
            Req::FilenameSub(substr) => format!("@f[{substr}]").into(),
            Req::PartOfSeq => "@seq".into(),
            Req::NTags(0) => "@untagged".into(),
            Req::NTags(n) => format!("@ntags[{n}]").into(),
        }
    }
}

#[derive(Error, Debug)]
pub enum ParseResolveError<'a> {
    #[error("{0}")]
    Parse(tagfilter_lang::ParseError<'a>),
    #[error("{0}")]
    ReqTransform(ReqTransformError<'a>),
}

impl<'a> From<ReqTransformError<'a>> for ParseResolveError<'a> {
    fn from(src: ReqTransformError<'a>) -> Self {
        Self::ReqTransform(src)
    }
}

impl<'a> From<tagfilter_lang::ParseError<'a>> for ParseResolveError<'a> {
    fn from(src: tagfilter_lang::ParseError<'a>) -> Self {
        Self::Parse(src)
    }
}

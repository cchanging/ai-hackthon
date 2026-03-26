// SPDX-License-Identifier: MPL-2.0

//! Serializable artifact model for the lockdep prototype.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A source file location attached to a reported fact.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

/// A structured lock-class identity derived from a MIR place.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LockClassKey {
    pub root: LockRootKey,
    #[serde(default)]
    pub projections: Vec<ProjectionKey>,
}

/// The root of one lock-class place.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LockRootKey {
    Global {
        def_path: String,
    },
    ReceiverArg {
        method_def_path: String,
        self_ty: String,
    },
    FnArg {
        fn_def_path: String,
        index: usize,
        ty: String,
    },
    Local {
        fn_def_path: String,
        index: usize,
        ty: String,
        #[serde(default)]
        origin: LocalOriginKey,
    },
    ReturnValue {
        fn_def_path: String,
        ty: String,
    },
}

/// Best-effort provenance for one MIR local used in a lock-class root.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LocalOriginKey {
    #[default]
    Unknown,
    AliasOf(Box<LockRootKey>),
    AggregateTemp {
        ty: String,
    },
    RefOfPlace {
        base: Box<LockRootKey>,
    },
}

/// One projection applied on top of a lock-class root.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProjectionKey {
    Deref {
        pointee_ty: String,
    },
    Field {
        owner_ty: String,
        field_name: String,
        field_index: usize,
    },
    Downcast {
        enum_ty: String,
        variant_name: String,
    },
    Index {
        index_local: usize,
    },
    ConstantIndex {
        offset: u64,
        min_length: u64,
        from_end: bool,
    },
    Subslice {
        from: u64,
        to: u64,
        from_end: bool,
    },
    OpaqueCast {
        ty: String,
    },
}

/// A normalized lock identity plus acquisition metadata.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LockInfoArtifact {
    pub class: LockClassKey,
    pub primitive: String,
    pub acquire: String,
    pub guard_behavior: String,
}

/// One lock acquire or release event inside a function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockEventArtifact {
    pub kind: String,
    pub context: String,
    pub lock: LockInfoArtifact,
    pub guard_local: Option<String>,
    pub location: Option<SourceLocation>,
}

/// One lock dependency edge extracted from a function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockEdgeArtifact {
    pub context: String,
    pub from: LockInfoArtifact,
    pub to: LockInfoArtifact,
    pub location: Option<SourceLocation>,
}

/// The IRQ-related usage bits observed for one lock mode.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LockUsageBitsArtifact {
    pub used_in_hardirq: bool,
    pub used_in_softirq: bool,
    pub used_with_hardirq_enabled: bool,
    pub used_with_hardirq_disabled: bool,
    pub used_with_softirq_enabled: bool,
    pub used_with_softirq_disabled: bool,
}

/// One witness site that set a specific IRQ-related usage bit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockUsageSiteArtifact {
    pub context: String,
    pub location: Option<SourceLocation>,
}

/// The aggregated IRQ-related usage state for one lock mode inside one function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockUsageStateArtifact {
    pub lock: LockInfoArtifact,
    pub bits: LockUsageBitsArtifact,
    pub first_hardirq_use: Option<LockUsageSiteArtifact>,
    pub first_softirq_use: Option<LockUsageSiteArtifact>,
    pub first_hardirq_enabled_use: Option<LockUsageSiteArtifact>,
    pub first_hardirq_disabled_use: Option<LockUsageSiteArtifact>,
    pub first_softirq_enabled_use: Option<LockUsageSiteArtifact>,
    pub first_softirq_disabled_use: Option<LockUsageSiteArtifact>,
}

/// All lockdep facts currently emitted for one function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionArtifact {
    pub def_path: String,
    pub location: Option<SourceLocation>,
    #[serde(default)]
    pub contexts: Vec<String>,
    #[serde(default)]
    pub lock_events: Vec<LockEventArtifact>,
    #[serde(default)]
    pub lock_edges: Vec<LockEdgeArtifact>,
    #[serde(default)]
    pub lock_usage_states: Vec<LockUsageStateArtifact>,
}

/// All lockdep facts emitted for one crate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisArtifact {
    pub crate_name: String,
    pub package_name: String,
    pub manifest_dir: String,
    pub target: Option<String>,
    pub is_primary_package: bool,
    pub functions: Vec<FunctionArtifact>,
}

impl fmt::Display for LockClassKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.root)?;
        for projection in &self.projections {
            write!(f, "{projection}")?;
        }
        Ok(())
    }
}

impl fmt::Display for LockRootKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Global { def_path } => write!(f, "global<{def_path}>"),
            Self::ReceiverArg {
                method_def_path,
                self_ty,
            } => write!(f, "self(method={method_def_path},ty={self_ty})"),
            Self::FnArg {
                fn_def_path,
                index,
                ty,
            } => write!(f, "arg(fn={fn_def_path},index={index},ty={ty})"),
            Self::Local {
                fn_def_path,
                index,
                ty,
                origin,
            } => write!(f, "local(fn={fn_def_path},index={index},ty={ty},origin={origin})"),
            Self::ReturnValue { fn_def_path, ty } => {
                write!(f, "return(fn={fn_def_path},ty={ty})")
            }
        }
    }
}

impl fmt::Display for LocalOriginKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => write!(f, "unknown"),
            Self::AliasOf(base) => write!(f, "alias({base})"),
            Self::AggregateTemp { ty } => write!(f, "aggregate<{ty}>"),
            Self::RefOfPlace { base } => write!(f, "ref({base})"),
        }
    }
}

impl fmt::Display for ProjectionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Deref { pointee_ty } => write!(f, "->*<{pointee_ty}>")?,
            Self::Field {
                owner_ty,
                field_name,
                field_index,
            } => write!(f, "->{owner_ty}::{field_name}#{field_index}")?,
            Self::Downcast {
                enum_ty,
                variant_name,
            } => write!(f, "->downcast({enum_ty}::{variant_name})")?,
            Self::Index { index_local } => write!(f, "[idx:{index_local}]")?,
            Self::ConstantIndex {
                offset,
                min_length,
                from_end,
            } => write!(
                f,
                "[const:{offset}:{min_length}:{}]",
                if *from_end { "end" } else { "start" }
            )?,
            Self::Subslice {
                from,
                to,
                from_end,
            } => write!(
                f,
                "[subslice:{from}:{to}:{}]",
                if *from_end { "end" } else { "start" }
            )?,
            Self::OpaqueCast { ty } => write!(f, "->cast<{ty}>")?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LocalOriginKey, LockClassKey, LockRootKey, ProjectionKey,
    };

    #[test]
    fn serializes_and_formats_boundary_lock_class_variants() {
        let key = LockClassKey {
            root: LockRootKey::Local {
                fn_def_path: "test::local_path".into(),
                index: 7,
                ty: "TempHolder".into(),
                origin: LocalOriginKey::RefOfPlace {
                    base: Box::new(LockRootKey::ReturnValue {
                        fn_def_path: "test::producer".into(),
                        ty: "SpinLock<u32>".into(),
                    }),
                },
            },
            projections: vec![
                ProjectionKey::Downcast {
                    enum_ty: "VariantLock".into(),
                    variant_name: "First".into(),
                },
                ProjectionKey::ConstantIndex {
                    offset: 1,
                    min_length: 2,
                    from_end: false,
                },
                ProjectionKey::Subslice {
                    from: 0,
                    to: 1,
                    from_end: true,
                },
                ProjectionKey::OpaqueCast {
                    ty: "dyn LockLike".into(),
                },
            ],
        };

        let json = serde_json::to_string(&key).unwrap();
        let decoded: LockClassKey = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, key);

        let display = key.to_string();
        assert!(display.contains("return(fn=test::producer,ty=SpinLock<u32>)"));
        assert!(display.contains("downcast(VariantLock::First)"));
        assert!(display.contains("[const:1:2:start]"));
        assert!(display.contains("[subslice:0:1:end]"));
        assert!(display.contains("cast<dyn LockLike>"));
    }

    #[test]
    fn formats_global_and_argument_roots() {
        let global = LockRootKey::Global {
            def_path: "crate::LOCK".into(),
        };
        let receiver = LockRootKey::ReceiverArg {
            method_def_path: "crate::Type::method".into(),
            self_ty: "crate::Type".into(),
        };
        let arg = LockRootKey::FnArg {
            fn_def_path: "crate::helper".into(),
            index: 2,
            ty: "SpinLock<u32>".into(),
        };

        assert_eq!(global.to_string(), "global<crate::LOCK>");
        assert!(receiver.to_string().contains("crate::Type::method"));
        assert_eq!(
            arg.to_string(),
            "arg(fn=crate::helper,index=2,ty=SpinLock<u32>)"
        );
    }
}

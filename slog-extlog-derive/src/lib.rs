//! Crate for autogenerating the [`ExtLoggable`](../slog_extlog/trait.ExtLoggable.html) trait.
//!
//! This massively simplifies the definition of extended logs - it is expected that most users
//! will automatically derive `ExtLoggable` for their log objects.  The derivation is done using
//! the `serde::Serialize` crate, for maximum flexibility and minimal new code.
//!
//! # External Logs
//! To autoderive the `ExtLoggable` trait:
//!
//!   - Import this crate and the `slog_extlog` crate with `#[macro_use]`.
//!   - Define a constant string named `CRATE_LOG_NAME` which uniquely identifies this crate in
//!     log identifiers.
//!   - Implement (usually by auto-deriving) `Clone` and `serde::Serialize` for the object.
//!   - Add `[#(derive(ExtLoggable)]` above the object, and a `LogDetails` attribute.
//!
//! The LogDetails must contain three key-value pairs, with all values quoted.
//!
//!   - `Level` must be a valid `slog::Level`.
//!   - `Text` must be a string literal.
//!   - `Id` must be an unsigned integer, which should uniquely identify the log within your crate.
//!
//! Your crate must also define a constant string named `CRATE_LOG_NAME`.  This is prefixed to the
//! log ID when the log is generated to
//!
//! # Values
//! If you wish a structure type to be usable as a log parameter, but not to be a log itself,
//! then this crate also allows derivation of the `slog::Value` trait - again, the type needs
//! to also implement `Clone` and `serde::Serialize`, and again these can nearly always be
//! auto-derived.
//!
//! # Limitations
//! Because the logging infrastructure can run asynchronously, all logged types must have static
//! lifetime and be `Send`.  This means no non-static references, and no lifetime parameters.
//!
//! # Statistics triggers
//! An external log can automatically trigger changes in statistic values by adding the
//! `StatTrigger` attribute to the log object. Like `LogDetails`, this attribute is a list of
//! `key=value` pairs, with the following keys.
//!
//! One `StatTrigger` attribute should be added to the log for each statistic it should update.
//!
//!   - `StatName` (mandatory) - The name of the statistic to change, as defined on the
//!     corresponding [`StatDefinition`](../slog_extlog/stats/struct.StatDefinition.html).
//!   - `Action` (mandatory) - one of: `Incr`, `Decr`, and `SetVal`, depending on whether this
//!     change triggers an increment, decrement, or set to an explicit value.
//!   - `Condition` (optional) - A condition, based on the log fields, for this stat to be changed.
//!     if not set, the stat is changed on every log.  The value of this parameter is an
//!     expression that returns a Boolean, and can use `self` for the current log object.
//!   - `Value` or `ValueFrom` - The value to increment/decrement/set.  One and only one
//!     of these must be provided.  `Value` for a fixed number, `ValueFrom` for an arbitrary
//!     expression to find the value that may return self.
//!   - `FixedGroups (optional)` - A comma-separated list of fixed tags to add to this statistic
//!     for this trigger - see below.
//!
//! ### Grouped (tagged) statistics)
//! Some statistics may be grouped with *tags*.  Tags can be defined in two ways.
//!
//!  - To add one or more *fixed* groups on a given statistic update, add an attribute to the
//!    `StatTrigger` of the form:  `FixedGroups = "<name>=<value>,<name2>=<value2>,...".
//!    The names must be the names of the tags within the statistic definition.
//!  - To add a *dynamic* tag, you can take the value from a single field in the log. To specify
//!    which field within the triggering log should be used for the group value, add a
//!    `#[StatGroup(StatName = "<name>")] attribute on the relevant field within the log, where
//!    `<name>` is the relevant statistic name.
//!    The name of the group within the statistic definition *must* be the name of the field
//!    in the log structure.
//!
//! **WARNING** - be careful with tagging where there can be large numbers of values - each seen
//! value for the tag generates a new statistic, which is tracked forever once it is seen.
//! Tags are most effective when used for fields that take a small number of values and where
//! the set of values changes infrequently.  Examples might be the type of remote client, or an
//! error code.
//!
//! # Example
//! Deriving `Value` and `ExtLoggable` for some simple objects.
//!
//! ```
//! use slog_extlog_derive::{ExtLoggable, SlogValue};
//! use serde::Serialize;
//!
//! #[derive(Clone, Serialize, SlogValue)]
//! enum FooRspCode {
//!     Success,
//!     InvalidUser,
//! }
//!
//! // The prefix to add to all log identifiers.
//! const CRATE_LOG_NAME: &'static str = "FOO";
//!
//! #[derive(Clone, Serialize, ExtLoggable)]
//! #[LogDetails(Id="101", Text="Foo Request received", Level="Info")]
//! struct FooReqRcvd;
//!
//! #[derive(Clone, Serialize, ExtLoggable)]
//! #[LogDetails(Id="103", Text="Foo response sent", Level="Info")]
//! struct FooRspSent(FooRspCode);
//!
//! # #[tokio::main]
//! # async fn main() { }
//! ```
//!
//! Defining some statistics from a single log.
//!
//! ```
//! use slog_extlog_derive::{ExtLoggable, SlogValue};
//! use serde::Serialize;
//!
//! use slog_extlog::{define_stats, stats, xlog};
//! use slog_extlog::stats::{Buckets, StatDefinition};
//! use slog::o;
//!
//! // The prefix to add to all log identifiers.
//! const CRATE_LOG_NAME: &'static str = "FOO";
//!
//! define_stats! {
//!    FOO_STATS = {
//!        // Some simple counters
//!        FooNonEmptyCount(Counter, "FOO-1001", "Count of non-empty Foo requests", []),
//!        FooTotalBytesByUser(Counter, "FOO-1002",
//!                            "Total size of all Foo requests per user", ["user", "request_type"])
//!    }
//! }
//!
//! #[derive(Clone, Serialize, ExtLoggable)]
//! #[LogDetails(Id="101", Text="Foo Request received", Level="Info")]
//! #[StatTrigger(StatName="FooNonEmptyCount", Action="Incr", Condition="self.bytes > 0", Value="1")]
//! #[StatTrigger(StatName="FooTotalBytesByUser", Action="Incr", ValueFrom="self.bytes", FixedGroups="request_type=Foo")]
//! struct FooReqRcvd {
//!   // The number of bytes in the request
//!   bytes: usize,
//!   // The user for the request.
//!   #[StatGroup(StatName = "FooTotalBytesByUser")]
//!   user: String
//! }
//!
//! # #[tokio::main]
//! async fn main() {
//!   // Create the logger using whatever log format required.
//!   let slog_logger = slog::Logger::root(slog::Discard, o!());
//!   let logger = stats::StatsLoggerBuilder::default()
//!       .with_stats(vec![FOO_STATS])
//!       .fuse(slog_logger);
//!
//!   // Now all logs of `FooReqRcvd` will increment the `FooNonEmptyCount` and
//!   // `FooTotalBytesByUser` stats...
//!
//!   xlog!(logger, FooReqRcvd { bytes: 42, user: "ArthurDent".to_string()});
//! }
//!
//! ```

// Copyright 2017 Metaswitch Networks

// LCOV_EXCL_START
// We cannot get coverage for procedural macros as they run at compile time.

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use slog::Level;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum StatTriggerAction {
    Increment,
    Decrement,
    SetValue,
    Ignore,
}

impl FromStr for StatTriggerAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Incr" => Ok(StatTriggerAction::Increment),
            "Decr" => Ok(StatTriggerAction::Decrement),
            "SetVal" => Ok(StatTriggerAction::SetValue),
            "None" => Ok(StatTriggerAction::Ignore),
            s => Err(format!("Unknown StatTrigger action {}", s)),
        }
    }
}

// Mappings of which fields in a struct are used as `StatGroup` or `BucketBy` for each statistic.
// Note that multiple fields can be used as `StatGroup`s for a given statistic, but only one can be
// a bucketing value.
#[derive(Default)]
struct FieldReferences {
    stat_group_refs: HashMap<String, HashSet<syn::Ident>>,
    bucket_by_ref: HashMap<String, syn::Ident>,
}

// Info about a statistic trigger
#[derive(Debug)]
struct StatTriggerData {
    id: syn::Ident,
    condition_body: syn::Expr,
    action: StatTriggerAction,
    val: syn::Expr,
    fixed_groups: HashMap<String, String>,
    field_groups: HashSet<syn::Ident>,
    bucket_by: Option<syn::Ident>,
}

impl StatTriggerData {
    // Returns the match case to pick this trigger out of the set of stats for an event.  This
    // should match a `StatDefinitionTagged` if the ID (as a string) matches the searched-for value
    // _and_ the provided `stat_id` matches the fixed tags for this trigger.
    //
    // This allows things like:
    //
    // ```ignore
    // #[StatTrigger(StatName="FooEventCounts", Action="Incr", Value=1, FixedGroups="Multiplier=1")]
    // #[StatTrigger(StatName="FooEventCounts", Action="Incr", Value=2, FixedGroups="Multiplier=2")]
    // struct FooEvent;
    // ```
    //
    // Which modify the same statistic twice with different values/actions/condititions so long as
    // they also have different FixedGroups.
    //
    // "<name>" if <stat_id_binding>.fixed_fields.any(|(k, v)|
    fn stat_lookup_case(&self, stat_id_binding: &syn::Ident) -> proc_macro2::TokenStream {
        let id = &self.id.to_string();
        let fixed_groups = self.fixed_groups.iter().map(|(k, v)| quote! { (#k, #v) });
        quote! { #id if #stat_id_binding.has_fixed_groups(&[#(#fixed_groups,)*]) }
    }
}

/// Generate implementations of the `slog::Value` trait.
///
/// Do not call this function directly.  Use `#[derive]` instead.
#[proc_macro_derive(SlogValue)]
pub fn slog_value(input: TokenStream) -> TokenStream {
    // Parse the type definition.
    let ast = syn::parse(input).unwrap();

    // Build the appropriate implementations
    let slog_serde_value = impl_slog_serde_value(&ast);
    let slog_value = impl_slog_value(&ast);

    // Emit all the trait impls
    quote! {
        #slog_serde_value
        #slog_value
    }
    .into()
}

/// Generate implementations of the [`ExtLoggable`](../slog_extlog/trait.ExtLoggable.html) trait.
///
/// Do not call this function directly.  Use `#[derive]` instead.
#[proc_macro_derive(
    ExtLoggable,
    attributes(LogDetails, FixedFields, StatTrigger, StatGroup, BucketBy)
)]
pub fn loggable(input: TokenStream) -> TokenStream {
    // Parse the type definition.
    let ast = syn::parse(input).unwrap();

    // Build the implementations of the various traits
    let ext_loggable = impl_ext_loggable(&ast);
    let slog_value = impl_slog_value(&ast);
    let slog_serde_value = impl_slog_serde_value(&ast);
    let stats_trigger = impl_stats_trigger(&ast);

    // Emit all the trait impls
    quote! {
        #ext_loggable
        #slog_value
        #slog_serde_value
        #stats_trigger
    }
    .into()
}

/// Build simple impl of slog::Value for the input struct
fn impl_slog_value(ast: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    // Generate implementations of slog::Value and slog::SerdeValue, for the purposes of
    // serialization.
    quote! {
        impl #impl_generics slog::SerdeValue for #name #ty_generics #where_clause {
            /// Convert into a serde object.
            fn as_serde(&self) -> &slog_extlog::erased_serde::Serialize {
                self
            }

            /// Convert to a boxed value that can be sent across threads.  This needs to handle
            /// converting the structure even if its lifetimes etc are not static.
            ///
            /// This enables functionality like `slog-async` and similar.
            fn to_sendable(&self) -> Box<dyn slog::SerdeValue + Send + 'static> {
                Box::new(self.clone())
            }
        }
    }
}

/// Build simple impl of slog::Value for the input struct
fn impl_slog_serde_value(ast: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    quote! {
        impl #impl_generics slog::Value for #name #ty_generics #where_clause {
            fn serialize(&self,
                         _record: &slog::Record,
                         key: slog::Key,
                         serializer: &mut slog::Serializer) -> slog::Result {
                serializer.emit_serde(key, self)

            }
        }
    }
}

/// Build an implementation of `slog_extlog::stats::StatTrigger` for the struct
fn impl_stats_trigger(ast: &syn::DeriveInput) -> proc_macro2::TokenStream {
    // Get the statname -> field references
    let field_references = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(named),
        ..
    }) = &ast.data
    {
        collate_field_references(named)
    } else {
        FieldReferences::default()
    };

    // Get stat triggering details.
    let stat_triggers = collect_stat_triggers(&ast.attrs, field_references);

    // Build up the return value for the `stat_list` method.
    //
    // StatDefinitionTagged { defn: <id>, fixed_tags: &[(<tag_name>, <tag_value>), ...] }
    let stat_ids = stat_triggers.iter().map(|trigger| {
        // Convert the fixed tags to 2-tuples
        let tag_pairs = trigger
            .fixed_groups
            .iter()
            .map(|(k, v)| quote! { (#k, #v) });
        let id = &trigger.id;
        quote! {
           slog_extlog::stats::StatDefinitionTagged { defn: &#id, fixed_tags: &[#(#tag_pairs),*] }
        }
    });
    let stat_ids_len = stat_triggers.len();

    // Use this binding to name the passed in StatDefinitionTagged so everyone uses the same name.
    let stat_id = syn::Ident::new("stat_id", proc_macro2::Span::call_site());

    // Build up the return values for the `condition` match statements.
    //
    // <name> if <fixed fields match> => <condition_expr>
    let stat_conds = stat_triggers.iter().map(|trigger| {
        let condition_case = trigger.stat_lookup_case(&stat_id);
        let condition = &trigger.condition_body;
        quote! { #condition_case => #condition }
    });

    // Build up the return values for the `change` match statements.
    //
    // <name> if <fixed fields match> => ChangeType::<type>(<value> as _)
    let stat_changes = stat_triggers.iter().map(|trigger| {
        let change_case = trigger.stat_lookup_case(&stat_id);
        let value_expr = &trigger.val;
        let change = match trigger.action {
            StatTriggerAction::Increment => quote! {
               Some(slog_extlog::stats::ChangeType::Incr((#value_expr) as usize))
            },
            StatTriggerAction::Decrement => quote! {
                Some(slog_extlog::stats::ChangeType::Decr((#value_expr) as usize))
            },
            StatTriggerAction::SetValue => quote! {
                Some(slog_extlog::stats::ChangeType::SetTo((#value_expr) as isize))
            },
            StatTriggerAction::Ignore => quote! { None },
        };
        quote! { #change_case => #change }
    });

    // Build up the tag (group) info for each stat.
    //
    // <stat_id> => match tag_name {
    //     "<tag_name>" => self.<tag_name>.to_string(),
    //     ...
    // }
    let stat_id_case_statements = stat_triggers.iter().map(|trigger| {
        let stat_id = &trigger.id.to_string();
        let stat_group_fields = trigger.field_groups.iter();
        let tag_case_statements = stat_group_fields.map(|field_name| {
            let field_name_as_str = field_name.to_string();
            quote! { #field_name_as_str => self.#field_name.to_string() }
        });
        quote! {
            #stat_id => {
                match tag_name {
                    #(#tag_case_statements,)*
                    _ => "".to_string()
                }
            }
        }
    });

    // Build up cases for the `bucket_value` match.
    //
    // <stat_id> => Some(self.<bucket_by> as f64)
    let stats_bucket_match_cases = stat_triggers.iter().flat_map(|t| {
        let id = &t.id.to_string();
        let bucket = t.bucket_by.as_ref();
        bucket.map(|b| {
            quote! { #id => Some(self.#b as f64) }
        })
    });

    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    quote! {
        impl #impl_generics slog_extlog::stats::StatTrigger for #name #ty_generics #where_clause {
            fn stat_list(&self) -> &[slog_extlog::stats::StatDefinitionTagged] {
                static STAT_LIST: [slog_extlog::stats::StatDefinitionTagged; #stat_ids_len] = [#(#stat_ids),*];
                &STAT_LIST
            }

            /// Evaluate any condition that must be satisfied for this stat to change.
            ///
            /// Panics in the case when we get called for an unknown stat.
            fn condition(&self, #stat_id: &slog_extlog::stats::StatDefinitionTagged) -> bool {
                match #stat_id.defn.name() {
                    #(#stat_conds,)*
                    s => panic!("Condition requested for unknown stat {}", s)
                }
            }

            /// The details of the change to make for this stat, if `condition` returned true.
            ///
            /// Panics in the case when we get called for an unknown stat.
            fn change(
                &self,
                #stat_id: &slog_extlog::stats::StatDefinitionTagged
            ) -> Option<slog_extlog::stats::ChangeType> {
                match #stat_id.defn.name() {
                    #(#stat_changes,)*
                    s => panic!("Change requested for unknown stat {}", s)
                }
            }

            /// Provide the value for the requested StatGroup dimension for this instance of the
            /// log.
            fn tag_value(
                &self,
                #stat_id: &slog_extlog::stats::StatDefinitionTagged,
                tag_name: &'static str
            ) -> String {
                // If this tag is in the fixed list, use the value provided.
                // Otherwise, call out to the trigger's value.
                if let Some(v) = #stat_id.fixed_tags.iter().find(|name| tag_name == name.0) {
                    v.1.to_string()
                } else {
                    match #stat_id.defn.name() {
                        #(#stat_id_case_statements,)*
                        _ => "".to_string(),
                    }
                }
            }

            /// The value to be used to sort the stat event into buckets (if appropriate)
            fn bucket_value(
                &self,
                #stat_id: &slog_extlog::stats::StatDefinitionTagged
            ) -> Option<f64> {
                match #stat_id.defn.name() {
                    #(#stats_bucket_match_cases,)*
                    _ => None,
                }
            }
        }
    }
}

/// The core `ExtLoggable` macro body, this expands a struct of the form:
///
/// ```ignore,rust
/// #[derive(ExtLoggable)]
/// #[LogDetails(Id = 2, Level = "Warn", Text = "Soemthing cool happened")]
/// #[FixedFields(key1="value1", key2="value2")] // Can be repeated
/// struct Foo {
///   field: String,
/// }
/// ```
fn impl_ext_loggable(ast: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    // Get the LogDetails attribute (singular, multiple such attributes are an error)
    let log_details = ast
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("LogDetails"))
        .collect::<Vec<_>>();
    if log_details.is_empty() {
        panic!("Unable to find LogDetails attribute");
    } else if log_details.len() > 1 {
        panic!("Multiple LogDetails attributes found");
    }
    let log_details = log_details[0];

    // Parse out the Id/Level/Text for this log convert the log level back to its variant name
    // (which isn't the `to_str` or `to_short_str` name unfortunately).
    let (level, text, id) = parse_log_details(log_details);
    let level = match level {
        Level::Critical => "Critical",
        Level::Error => "Error",
        Level::Warning => "Warning",
        Level::Info => "Info",
        Level::Debug => "Debug",
        Level::Trace => "Trace",
    };
    let level = syn::Ident::new(level, proc_macro2::Span::call_site());

    // Collect the `FixedFields` for this log
    let fixed_fields = parse_fixed_fields(&ast.attrs);

    // Write out the implementation of ExtLoggable.
    quote! {
        impl #impl_generics slog_extlog::ExtLoggable for #name #ty_generics #where_clause {
            fn ext_log(&self, logger: &slog_extlog::stats::StatisticsLogger) {
                logger.update_stats(self);
                // Use a `FnValue` for the log ID so the format string is allocated only if the log
                // is actually written.  ideally, we'd like this to be compile-time allocated but
                // we can't yet pass const variables from the caller into the procedural macro...
                let id_val = slog::FnValue(|_| format!("{}-{}", CRATE_LOG_NAME, #id));
                slog::log!(logger, slog::Level::#level, "", #text; "log_id" => id_val, #(#fixed_fields, )* "details" => self)
            }
        }
    }
}

/// Parses a `LogDetails` attribute.  This attribute requires exactly three parameters, each with an
/// associated value:
///
/// * The `Id` attribute must be an unsigned integral Id for the log (can be provided as `Id = 12`
///   or `Id = "12"` for back-compatibility).
/// * The `Level` attribute must be a string naming a `slog::Level` (e.g. "Trace", "Debug", "Info",
///   etc.)  For back-compatibility, also allows "Warning" for "Warn".
/// * The `Text` attribute must be a string that will make up the `msg` attribute of the log.
///
/// ```ignore
/// #[LogDetails(Id = 12, Level = "Info", Text = "Something cool happened")]
/// ```
fn parse_log_details(attr: &syn::Attribute) -> (Level, String, u64) {
    // Make sure we get the three values we need from the attributes.
    let mut id = None;
    let mut level = None;
    let mut text = None;

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("Id") {
            if id.is_some() {
                panic!("Id attribute passed twice in LogDetails");
            }
            let value = meta
                .value()
                .expect("Id parameter of LogDetails needs a value");
            let value: syn::Lit = value
                .parse()
                .expect("Id parameter of LogDetails needs to be a literal");
            match value {
                syn::Lit::Str(s) => {
                    id = Some(s.value().parse::<u64>().expect(
                        "Invalid format for LogDetails - Id attribute must be an unsigned integer",
                    ))
                }
                syn::Lit::Int(i) => {
                    id = Some(i.base10_parse::<u64>().expect("Invalid format for LogDetails - Id attribute must be an unsigned integer"))
                }
                _ => panic!("Id parameter of LogDetails needs to be an unsigned integer (literal or string representation"),
            };
        } else if meta.path.is_ident("Level") {
            if level.is_some() {
                panic!("Level attribute passed twice in LogDetails");
            }
            let value = meta.value().expect("Level parameter of LogDetails needs a value");
            let value: syn::LitStr = value.parse().expect("Level parameter of LogDetails must be a string");
            let value = value.value();
            // We handle "Warning" specially - Level::from_str *used* to erroneously handle this as
            // it only did prefix matches, but now it requires exactly the word "Warn".
            level = Some(if value == "Warning" {
                Level::Warning
            } else {
                Level::from_str(&value).unwrap_or_else(|_| {
                    panic!("Invalid log level provided: {}", value)
                })
            });
        } else if meta.path.is_ident("Text") {
            if text.is_some() {
                panic!("Text attribute passed twice in LogDetails");
            }
            let value = meta.value().expect("Text parameter of LogDetails needs a value");
            let value: syn::LitStr = value.parse().expect("Text parameter of LogDetails must be a string");
            text = Some(value.value());
        } else {
            panic!("Unexpected key '{:?}' in LogDetails attribute", meta.path)
        }
        Ok(())
    }).unwrap();

    // We must now have exactly the elements we want.  Panic if not.
    (
        level.expect("No Level provided in LogDetails"),
        text.expect("No Text provided in LogDetails"),
        id.expect("No Id provided in LogDetails"),
    )
}

/// Parse `FixedFields` attributes from an attribute set.  These attributes must be of the form
/// `#[FixedFields(key="value",...)]` and all fixed fields are combined together (so putting
/// multiple fields in one attribute is equivalent to putting them in mulitple attributes).
fn parse_fixed_fields(
    attrs: &[syn::Attribute],
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("FixedFields"))
        .flat_map(|val| {
            let mut fixed_fields = Vec::new();
            val.parse_nested_meta(|meta| {
                let key = &meta.path.require_ident().unwrap().to_string();
                let value = meta
                    .value()
                    .unwrap_or_else(|_| panic!("Field {:?} in FixedFields must have a value", key));
                let value: syn::LitStr = value.parse().unwrap_or_else(|_| {
                    panic!(
                        "Field {:?} in FixedFields must have a literal string value",
                        key
                    )
                });
                fixed_fields.push(quote!(#key => #value));
                Ok(())
            })
            .unwrap();
            fixed_fields
        })
}

/// Collates `StatGroup` and `BucketBy` field attributes from a struct.  These attributes each
/// contain exactly one `StatName` parameter with a string value (that names the statistic they
/// apply to).
///
/// ```ignore
/// #[StatGroup(StatName="cool_event_count")]
/// #[BucketBy(StatName="cool_event_histogram")]
/// field: Type,
/// ```
fn collate_field_references(fields: &syn::FieldsNamed) -> FieldReferences {
    let mut field_refs = FieldReferences::default();
    for field in fields.named.iter() {
        for attr in field.attrs.iter() {
            enum RefType {
                StatGroup,
                BucketBy,
            }
            let ref_type = match format!("{}", attr.path().require_ident().unwrap()).as_str() {
                "StatGroup" => RefType::StatGroup,
                "BucketBy" => RefType::BucketBy,
                _ => continue,
            };

            let mut stat_name = None;
            attr.parse_nested_meta(|meta| {
                if !meta.path.is_ident("StatName") {
                    panic!(
                        "Unrecognised parameter '{:?}' in {:?} attribute",
                        meta.path,
                        attr.path()
                    );
                }
                let value = meta.value().expect("StatName parameter needs a value");
                let value: syn::LitStr = value
                    .parse()
                    .expect("StatName parameter must be a string literal");
                stat_name = Some(value.value());
                Ok(())
            })
            .unwrap();

            let stat_name = stat_name.expect("No `StatName` parameter provided");
            let field_name = field.ident.clone().unwrap(); // We're in a Namedfields, surely our fields
                                                           // have names?
            match ref_type {
                RefType::StatGroup => {
                    field_refs
                        .stat_group_refs
                        .entry(stat_name)
                        .or_default()
                        .insert(field_name);
                }
                RefType::BucketBy => match field_refs.bucket_by_ref.entry(stat_name.clone()) {
                    std::collections::hash_map::Entry::Occupied(_) => {
                        panic!("Multiple `BucketBy` attributes found for `{}`", stat_name)
                    }
                    std::collections::hash_map::Entry::Vacant(v) => {
                        v.insert(field_name);
                    }
                },
            }
        }
    }
    field_refs
}

/// Collects all the `StatTrigger` attributes for the struct.
fn collect_stat_triggers<'a>(
    attrs: impl IntoIterator<Item = &'a syn::Attribute>,
    field_refs: FieldReferences,
) -> Vec<StatTriggerData> {
    let mut stat_triggers = Vec::<StatTriggerData>::new();

    for attr in attrs
        .into_iter()
        .filter(|attr| attr.path().is_ident("StatTrigger"))
    {
        let stat_trigger = parse_stat_trigger(attr, &field_refs);
        stat_triggers.push(stat_trigger);
    }

    stat_triggers
}

/// Parses a `StatTrigger` attribute to a `StatTriggerData` object.  This attribute has various
/// parameters:
///
/// * `StatName` (mandatory) - The name of the statistic to modify
/// * `Action` (mandatory) - The operation to apply to the statistic (`Incr`, `Decr` or `SetVal`)
/// * `Value` or `ValueFrom` (exactly one must be present) - How to determine the value for the
///   operation (either provided as a literal `i64` or an expression to invoke that can access
///   `self`)
/// * `Condition` (optional) - An expression (that may reference `self`) that returns a boolean,
///   the statistic will not be updated if this expression returns false.  If omitted, the statistic
///   is always updated.
/// * `FixedGroups` (optional) - A comma-separated list of `<group_name>=<value>` to be converted
///   to parameterization of the statistic (see `define_stats!`)
fn parse_stat_trigger(attr: &syn::Attribute, field_refs: &FieldReferences) -> StatTriggerData {
    let mut id = None;
    let mut cond = None;
    let mut trigger_action = None;
    let mut trigger_value = None;
    let mut fixed_groups = HashMap::new();
    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("StatName") {
            let value = meta.value().unwrap();
            let value: syn::LitStr = value.parse().unwrap();
            id = Some(syn::Ident::new(&value.value(), value.span()));
        } else if meta.path.is_ident("Condition") {
            let value = meta.value().unwrap();
            let value: syn::LitStr = value.parse().unwrap();
            let value = syn::parse_str::<syn::Expr>(&value.value()).unwrap();
            cond = Some(value);
        } else if meta.path.is_ident("Action") {
            let value = meta.value().unwrap();
            let value: syn::LitStr = value.parse().unwrap();
            let value = StatTriggerAction::from_str(&value.value()).unwrap();
            trigger_action = Some(value)
        } else if meta.path.is_ident("Value") {
            let value = meta.value().unwrap();
            let value: syn::Lit = value.parse().unwrap();
            let value = match value {
                syn::Lit::Str(s) => s.value().parse::<i64>().unwrap(),
                syn::Lit::Int(i) => i.base10_parse::<i64>().unwrap(),
                _ => panic!("Invalid parameter for `Value` in `StatTrigger`"),
            };

            let value = proc_macro2::Literal::i64_unsuffixed(value);
            let value = syn::LitInt::from(value);
            let value = syn::Lit::from(value);
            let value = syn::ExprLit {
                attrs: Vec::new(),
                lit: value,
            };
            let value = syn::Expr::from(value);
            trigger_value = Some(value);
        } else if meta.path.is_ident("ValueFrom") {
            let value = meta.value().unwrap();
            let value: syn::LitStr = value.parse().unwrap();
            let value = syn::parse_str::<syn::Expr>(&value.value()).unwrap();
            trigger_value = Some(value);
        } else if meta.path.is_ident("FixedGroups") {
            let value = meta.value().unwrap();
            let value: syn::LitStr = value.parse().unwrap();
            let value = value.value();
            for group in value.split(',') {
                let mut split = group.splitn(2, '=');
                let group_name = split.next().expect("Invalid format for FixedGroups");
                let group_val = split.next().expect("Invalid format for FixedGroups");
                fixed_groups.insert(group_name.to_string(), group_val.to_string());
            }
        } else {
            panic!(
                "Unrecognised parameter `{:?}` in StatTrigger attribute",
                meta.path
            );
        }
        Ok(())
    })
    .unwrap();

    let id = id.expect("StatTrigger missing value for StatName");

    let stat_group_fields = field_refs
        .stat_group_refs
        .get(&id.to_string())
        .cloned()
        .unwrap_or_default();
    let bucket_by_field = field_refs.bucket_by_ref.get(&id.to_string()).cloned();

    StatTriggerData {
        id,
        // If no condition is provided, default to always passing.
        condition_body: cond.unwrap_or_else(|| syn::parse_quote!(true)),
        action: trigger_action.expect("StatTrigger missing value for Action"),
        val: trigger_value.expect("StatTrigger missing value for Value or ValueFrom"),
        fixed_groups,
        field_groups: stat_group_fields,
        bucket_by: bucket_by_field,
    }
}
// LCOV_EXCL_STOP

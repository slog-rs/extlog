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
//!           corresponding [`StatDefinition`](../slog_extlog/stats/struct.StatDefinition.html).
//!   - `Action` (mandatory) - one of: `Incr`, `Decr`, and `SetVal`, depending on whether this
//!        change triggers an increment, decrement, or set to an explicit value.
//!   - `Condition` (optional) - A condition, based on the log fields, for this stat to be changed.
//!      if not set, the stat is changed on every log.  The value of this parameter is an
//!        expression that returns a Boolean, and can use `self` for the current log object.
//!   - `Value` or `ValueFrom` - The value to increment/decrement/set.  One and only one
//!   of these must be provided.  `Value` for a fixed number, `ValueFrom` for an arbitrary
//!   expression to find the value that may return self.
//!   - `FixedGroups (optional)` - A comma-separated list of fixed tags to add to this statistic
//!   for this trigger - see below.
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
//!                            "Total size of all Foo requests per user", ["user"])
//!    }
//! }
//!
//! #[derive(Clone, Serialize, ExtLoggable)]
//! #[LogDetails(Id="101", Text="Foo Request received", Level="Info")]
//! #[StatTrigger(StatName="FooNonEmptyCount", Action="Incr",
//!               Condition="self.bytes > 0", Value="1")]
//! #[StatTrigger(StatName="FooTotalBytesByUser", Action="Incr", ValueFrom="self.bytes")]
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

#![recursion_limit = "128"]

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use slog::Level;
use std::collections::HashMap;
use std::str::FromStr;

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
            s => Err(format!("Unknown action {}", s)),
        }
    }
}

enum StatTriggerValue {
    Fixed(i64),
    Expr(Box<syn::Expr>),
}

// Info about a statistic trigger
struct StatTriggerData {
    id: syn::Ident,
    condition_body: syn::Expr,
    action: StatTriggerAction,
    val: StatTriggerValue,
    fixed_groups: HashMap<String, String>,
    field_groups: Vec<syn::Ident>,
    bucket_by: Option<syn::Ident>,
}

/// Generate implementations of the `slog::Value` trait.
///
/// Do not call this function directly.  Use `#[derive]` instead.
#[proc_macro_derive(SlogValue)]
pub fn slog_value(input: TokenStream) -> TokenStream {
    // Parse the type definition.
    let ast = syn::parse(input).unwrap();

    // Build the impl
    let gen = impl_value_traits(&ast);

    // Return the generated impl
    TokenStream::from(gen)
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

    // Build the impl
    let gen = impl_loggable(&ast);

    // Return the generated impl
    TokenStream::from(gen)
}

// Actually build impls of Value and SerdeValue.
fn impl_value_traits(ast: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    // Need several copies of the lifetimes to avoid using moved values in the macro.
    let lifetimes = ast.generics.lifetimes();
    let lifetimes_2 = ast.generics.lifetimes();
    let lifetimes_3 = ast.generics.lifetimes();
    let lifetimes_4 = ast.generics.lifetimes();
    let ty_params: Vec<_> = ast.generics.type_params().collect();

    let (tys, bounds) = get_types_bounds(&ty_params);
    // Tedious clones so we can iterate over them in quote macros multiple times.
    let tys_2 = tys.clone();
    let tys_3 = tys.clone();
    let tys_4 = tys.clone();
    let tys_5 = tys.clone();
    let tys_6 = tys.clone();
    let bounds2 = bounds.clone();

    // Generate implementations of slog::Value and slog::SerdeValue, for the purposes of
    // serialization.
    quote! {
        impl <#(#lifetimes,)* #(#tys),*> slog::SerdeValue for #name<#(#lifetimes_2,)* #(#tys_2),*>
            #(where #tys_3: #(#bounds + )* serde::Serialize + slog::Value),*  {

            /// Convert into a serde object.
            fn as_serde(&self) -> &slog_extlog::erased_serde::Serialize {
                self
            }

            /// Convert to a boxed value that can be sent across threads.  This needs to handle
            /// converting the structure even if its lifetimes etc are not static.
            ///
            /// This enables functionality like `slog-async` and similar.
            fn to_sendable(&self) -> Box<slog::SerdeValue + Send + 'static> {
                Box::new(self.clone())
            }
        }

        impl<#(#lifetimes_3,)* #(#tys_4),*> slog::Value for #name<#(#lifetimes_4,)* #(#tys_5),*>
            #(where #tys_6: #(#bounds2 + )* slog::Value),* {
            fn serialize(&self,
                         _record: &slog::Record,
                         key: slog::Key,
                         serializer: &mut slog::Serializer) -> slog::Result {
                serializer.emit_serde(key, self)

            }
        }
    }
}

fn get_types_bounds<'a>(
    ty_params: &[&'a syn::TypeParam],
) -> (Vec<&'a syn::Ident>, Vec<Vec<&'a syn::TraitBound>>) {
    let tys: Vec<&syn::Ident> = ty_params.iter().map(|param| &param.ident).collect();
    let bounds = ty_params
        .iter()
        .map(|p| {
            p.bounds
                .iter()
                .filter_map(|t| {
                    if let syn::TypeParamBound::Trait(ref tr) = *t {
                        Some(tr)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    (tys, bounds)
}

fn impl_stats_trigger(ast: &syn::DeriveInput) -> proc_macro2::TokenStream {
    // Get stat triggering details.
    let triggers = ast
        .attrs
        .iter()
        .filter(|a| a.path.is_ident("StatTrigger"))
        .map(|a| match a.parse_meta() {
            Ok(syn::Meta::List(metalist)) => {
                let nested = metalist.nested.iter();
                parse_stat_trigger(nested, &ast.data)
            }
            _ => panic!("Invalid format for #[StatTrigger(attr=\"val\")]"),
        })
        .collect::<Vec<_>>();

    // Build up the return value for the `stat_list` method.
    let stat_ids = triggers
        .iter()
        .map(|t| {
            let (keys, vals): (Vec<_>, Vec<_>) = t
                .fixed_groups
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .unzip();
            let id = &t.id;
            quote! {
               slog_extlog::stats::StatDefinitionTagged { defn: &#id, fixed_tags: &[#( (#keys, #vals) ),*] }
            }
        })
        .collect::<Vec<_>>();

    // Build up the input match statements value for the `condition` method.
    let stat_ids_cond = triggers
        .iter()
        .map(|t| {
            let (keys, vals): (Vec<_>, Vec<_>) = t
                .fixed_groups
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .unzip();
            let id = t.id.to_string();
            // The horrific chicanery here is because match guards can't use mutable borrows, so
            // `any` and `find` and such methods can't be used.
            quote! {
                #id if (true #(&& stat_id.fixed_tags.iter().filter(|tag| tag.0 == #keys && tag.1 == #vals).count() != 0) *)
            }
        })
        .collect::<Vec<_>>();

    // Build up the return values for those match statements.
    let stat_conds = triggers
        .iter()
        .map(|t| &t.condition_body)
        .collect::<Vec<_>>();

    // Build up the return values for those match statements.
    let stat_changes = triggers
        .iter()
        .map(|t| {
            let val = &(match t.val {
                StatTriggerValue::Fixed(v) => quote! {#v as usize},
                StatTriggerValue::Expr(ref e) => quote! {(#e) as usize },
            });
            match t.action {
                StatTriggerAction::Increment => quote! {
                   Some(slog_extlog::stats::ChangeType::Incr(#val))
                },
                StatTriggerAction::Decrement => quote! {
                    Some(slog_extlog::stats::ChangeType::Decr(#val))
                },
                StatTriggerAction::SetValue => quote! {
                    Some(slog_extlog::stats::ChangeType::SetTo(#val as isize))
                },
                StatTriggerAction::Ignore => quote! { None },
            }
        })
        .collect::<Vec<_>>();

    // Build up the tag (group) info for each stat.
    let mut stats_groups = quote! {};
    for t in &triggers {
        let id = &t.id.to_string();
        let dyn_groups = t.field_groups.clone();
        let dyn_groups_str = dyn_groups
            .clone()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        stats_groups = quote! { #stats_groups
            #id => { match tag_name {
              #(#dyn_groups_str => self.#dyn_groups.to_string(),)*
                _ => "".to_string() }
            },
        }
    }

    // Build up the bucket info for each stat.
    let mut stats_buckets = quote! {};
    for t in &triggers {
        let id = &t.id.to_string();
        let bucket = t.bucket_by.clone();
        if let Some(bucket) = bucket {
            stats_buckets = quote! { #stats_buckets
                #id => Some(self.#bucket as f64),
            }
        }
    }

    // Tweak to ensure we avoid unused variable warnings in `get_tag_value()`.
    let tag_name_ident = if !triggers.is_empty() {
        quote! { tag_name }
    } else {
        quote! { _tag_name }
    };

    let name = &ast.ident;
    let lifetimes = ast.generics.lifetimes();
    let lifetimes_2 = ast.generics.lifetimes();
    let ty_params: Vec<_> = ast.generics.type_params().collect();

    let (tys, bounds) = get_types_bounds(&ty_params);
    let tys_2 = tys.clone();
    let tys_3 = tys.clone();

    // Create a new identifier for the list of stats, so we can make the list globally static.
    let stat_ids_name = format_ident!("STATS_LIST_{}", name.to_string().to_uppercase());

    quote! {
        static #stat_ids_name: &'static [slog_extlog::stats::StatDefinitionTagged] = &[#(#stat_ids),*];
        impl<#(#lifetimes,)* #(#tys),*> slog_extlog::stats::StatTrigger
            for #name<#(#lifetimes_2,)* #(#tys_2),*>
        #(where #tys_3: #(#bounds + )* slog::Value),*{

            fn stat_list(
                &self) -> &'static[slog_extlog::stats::StatDefinitionTagged] {
                #stat_ids_name
            }

            /// The condition that must be satisfied for this stat to change.
            /// Panic in the case when we get called for an unknown stat.
            fn condition(&self, stat_id: &slog_extlog::stats::StatDefinitionTagged) -> bool {
                match stat_id.defn.name() {
                    #(#stat_ids_cond => #stat_conds,)*
                    s => panic!("Condition requested for unknown stat {}", s)
                }

            }
            /// The details of the change to make for this stat, if `condition` returned true.
            fn change(&self,
                      stat_id: &slog_extlog::stats::StatDefinitionTagged) ->
                      Option<slog_extlog::stats::ChangeType> {
                match stat_id.defn.name() {
                    #(#stat_ids_cond => #stat_changes,)*
                    s => panic!("Change requested for unknown stat {}", s)
                }
            }

            /// The fields that provide the grouped values for this stat
            fn tag_value(&self,
                         stat_id: &slog_extlog::stats::StatDefinitionTagged,
                         #tag_name_ident: &'static str) -> String {

                // If this tag is in the fixed list, use the value provided.
                // Otherwise, call out to the trigger's value.
                if let Some(v) = stat_id.fixed_tags.iter().find(|name| #tag_name_ident == name.0) {
                    v.1.to_string()
                } else {
                    match stat_id.defn.name() {
                        #stats_groups
                        _ => "".to_string(),
                    }
                }
            }

            /// The value to be used to sort the stat into buckets
            fn bucket_value(&self,
                         stat_id: &slog_extlog::stats::StatDefinitionTagged) -> Option<f64> {
                match stat_id.defn.name() {
                    # stats_buckets
                    _ => None,
                }
            }
        }
    }
}

fn impl_loggable(ast: &syn::DeriveInput) -> proc_macro2::TokenStream {
    let name = &ast.ident;
    let lifetimes = ast.generics.lifetimes();
    let lifetimes_2 = ast.generics.lifetimes();
    let ty_params: Vec<_> = ast.generics.type_params().collect();

    let (tys, bounds) = get_types_bounds(&ty_params);
    let tys_2 = tys.clone();
    let tys_3 = tys.clone();

    // Get the log details from the attribute.
    let vals = ast
        .attrs
        .iter()
        .filter(|a| a.path.is_ident("LogDetails"))
        .collect::<Vec<_>>();
    if vals.len() != 1 {
        panic!("Unable to find LogDetails attribute, or multiple LogDetails supplied")
    }
    let (level, text, id) = match vals[0].parse_meta() {
        Ok(syn::Meta::List(metalist)) => {
            let nested: Vec<_> = metalist.nested.iter().collect();
            parse_log_details(&nested)
        }
        _ => panic!("Invalid format for #[LogDetails(id, level, text)]"),
    };

    // Get the fixed fields from the attribute.
    let fields = ast
        .attrs
        .iter()
        .filter(|a| a.path.is_ident("FixedFields"))
        .flat_map(|val| {
            let meta = val.parse_meta();
            match meta {
                Ok(syn::Meta::List(metalist)) => metalist
                    .nested
                    .iter()
                    .map(parse_fixed_field)
                    .map(|(key, value)| quote!( #key => #value ))
                    .collect::<Vec<_>>(),
                _ => panic!("Invalid format for #[FixedFields(key = value)]"),
            }
        });

    // Implement the relevant traits for the structure parameters to be used as key-value pairs.
    let kv_gen = impl_value_traits(ast);

    // Generate the actual log call based on the provided level.
    let match_gen = match level {
        Level::Critical => {
            quote! { slog::crit!(logger, #text; "log_id" => id_val, #(#fields, )* "details" => self) }
        }
        Level::Error => {
            quote! { slog::error!(logger, #text; "log_id" => id_val, #(#fields, )* "details" =>  self) }
        }
        Level::Warning => {
            quote! { slog::warn!(logger, #text; "log_id" => id_val, #(#fields, )* "details" => self) }
        }
        Level::Info => {
            quote! { slog::info!(logger, #text; "log_id" => id_val, #(#fields, )* "details" => self) }
        }
        Level::Debug => {
            quote! { slog::debug!(logger, #text; "log_id" => id_val, #(#fields, )* "details" => self) }
        }
        Level::Trace => {
            quote! { slog::trace!(logger, #text; "log_id" => id_val, #(#fields, )* "details" => self) }
        }
    };

    let stat_gen = impl_stats_trigger(ast);

    // Write out the implementation of ExtLoggable.
    quote! {
        impl<#(#lifetimes,)* #(#tys),*> slog_extlog::ExtLoggable
            for #name<#(#lifetimes_2,)* #(#tys_2),*>
        #(where #tys_3: #(#bounds + )* slog::Value),*{

            fn ext_log(&self, logger: &slog_extlog::stats::StatisticsLogger) {
                logger.update_stats(self);
                // Use a `FnValue` for the log ID so the format string is allcoated only if the log
                // is actually written.  dieally, we'd like this to be compile-time allocated but
                // we can't yet pass const variables from the caller into the procedural macro...
                let id_val = slog::FnValue(|_| format!("{}-{}", CRATE_LOG_NAME, #id));
                #match_gen
            }
        }
        // Add the implementations of the traits we generated above.
        #kv_gen

        #stat_gen
    }
}

// Parses the LogDetails attribute.
fn parse_log_details(attr_val: &[&syn::NestedMeta]) -> (Level, String, u64) {
    if attr_val.len() != 3 {
        panic!("Must have exactly 3 parameters for LogDetails - ID, level, text")
    }

    // Make sure we get the three values we need from the attributes.  Use Options to avoid
    // issues with uninitialized variables.
    let mut id = None;
    let mut level = None;
    let mut text = None;

    for attr in attr_val {
        match *attr {
            // Attributes can have many forms.  We expect these to be NameValue,
            // of the form name="val".  Anything else is invalid.
            //
            // This branch of code will ensure that Id, Text and Level end up as as Some(value) if
            // one was provided.
            syn::NestedMeta::Meta(syn::Meta::NameValue(ref name_value)) => {
                // Check for one of the three keys we care about - Id, Text, Level.
                if name_value.path.is_ident("Id") {
                    // The ID must parse to a valid unsigned integer.
                    id = match name_value.lit {
                        syn::Lit::Str(ref s) => Some(s.value().parse::<u64>().expect(
                            "Invalid format for LogDetails - Id attribute must be an \
                             unsigned integer",
                        )),
                        _ => panic!(
                            "Invalid format for LogDetails - Id attribute must be a \
                             string-quoted unsigned integer"
                        ),
                    };
                } else if name_value.path.is_ident("Level") {
                    level = match name_value.lit {
                        syn::Lit::Str(ref s) => {
                            let s = s.value();
                            // Level must be a valid slog::Level.  Generate an error if not.
                            Some(
                                // We handle "Warning" specially - Level::from_str *used* to
                                // erroneously handle this as it only did prefix matches, but
                                // now it requires exactly the word "Warn".
                                if s == "Warning" {
                                    Level::Warning
                                } else {
                                    Level::from_str(&s).unwrap_or_else(|_| {
                                        panic!("Invalid log level provided: {}", s)
                                    })
                                },
                            )
                        }
                        _ => panic!(
                            "Invalid format for LogDetails - Level attribute must be a \
                             string-quoted slog::Level"
                        ),
                    };
                } else if name_value.path.is_ident("Text") {
                    text = match name_value.lit {
                        // Text has no restrictions other than being a string literal.
                        syn::Lit::Str(ref s) => Some(s.value().clone()),
                        _ => panic!(
                            "Invalid format for LogDetails - Text attribute must be a \
                             string literal"
                        ),
                    };
                } else {
                    panic!("Unknown attribute in LogDetails")
                }
            }
            _ => panic!("Invalid format for LogDetails - parameters must be key-value pairs"),
        }
    }

    // We should now have exactly the 3 elements we want as Some(X).  Panic if not.
    (
        level.expect("No Level provided in LogDetails"),
        text.expect("No Text provided in LogDetails"),
        id.expect("No Id provided in LogDetails"),
    )
}

// Parses the FixedField attribute.
fn parse_fixed_field(attr_val: &syn::NestedMeta) -> (String, String) {
    match *attr_val {
        // Attributes can have many forms.  We expect these to be NameValue,
        // of the form name="val".  Anything else is invalid.
        syn::NestedMeta::Meta(ref item) => match *item {
            syn::Meta::NameValue(ref name_value) => {
                let ident = name_value
                    .path
                    .get_ident()
                    .expect("Invalid format for FixedFields");
                if let syn::Lit::Str(ref s) = name_value.lit {
                    (ident.to_string(), s.value())
                } else {
                    panic!("Invalid format for FixedFields - value must be a string");
                }
            }
            _ => panic!("Invalid format for FixedFields - value must be a string"),
        },
        _ => panic!("Invalid format for FixedFields - parameters must be key-value pairs"),
    }
}

// Check whether a field's attributes include  "StatName = <id>"
fn is_attr_stat_id(attr: &syn::Attribute, id: &syn::Ident) -> bool {
    match attr.parse_meta() {
        // We only care about the case where this is a list of key-value type attributes.
        Ok(syn::Meta::List(ref list)) => list.nested.iter().any(|inner| {
            if let syn::NestedMeta::Meta(syn::Meta::NameValue(ref name_value)) = *inner {
                if let syn::Lit::Str(ref s) = name_value.lit {
                    let parsed_value = format_ident!("{}", s.value());
                    name_value.path.is_ident("StatName") && &parsed_value == id
                } else {
                    false
                }
            } else {
                false
            }
        }),
        _ => false,
    }
}

// Parses the StatTrigger attribute.
fn parse_stat_trigger<'a>(
    attr_val: impl Iterator<Item = &'a syn::NestedMeta>,
    body: &syn::Data,
) -> StatTriggerData {
    let mut id = None;
    let mut cond = None;
    let mut action = None;
    let mut value = None;
    let mut fixed_groups = HashMap::new();

    for attr in attr_val {
        let (name, val) = match *attr {
            // Attributes can have many forms.  We expect these to be NameValue,
            // of the form name="val".  Anything else is invalid.
            syn::NestedMeta::Meta(ref item) => match *item {
                syn::Meta::NameValue(ref name_value) => {
                    let ident = name_value
                        .path
                        .get_ident()
                        .expect("Invalid format for StatTrigger");
                    if let syn::Lit::Str(ref s) = name_value.lit {
                        (ident.to_string(), s.value())
                    } else {
                        panic!("Invalid format for StatTrigger - value must be a string");
                    }
                }
                _ => panic!("Invalid format for StatTrigger - value must be a string"),
            },
            _ => panic!("Invalid format for StatTrigger - parameters must be key-value pairs"),
        };

        match name.as_ref() {
            "StatName" => id = Some(format_ident!("{}", val)),
            "Condition" => {
                let token_stream: TokenStream = val.parse().unwrap();
                cond = Some(
                    syn::parse(token_stream).expect("Could not parse condition in StatTrigger"),
                );
            }
            "Action" => {
                action =
                    Some(StatTriggerAction::from_str(&val).expect("Invalid Action in StatTrigger"))
            }
            "Value" => {
                value = Some(StatTriggerValue::Fixed(
                    val.parse::<i64>().expect("Invalid Value in StatTrigger"),
                ))
            }
            "ValueFrom" => {
                let token_stream: TokenStream = val.parse().unwrap();
                value = Some(StatTriggerValue::Expr(
                    syn::parse(token_stream).expect("Invalid ValueFrom in StatTrigger"),
                ));
            }
            "FixedGroups" => {
                // Split the value
                let groups = val.split(',');
                for group in groups {
                    let mut split = group.splitn(2, '=');
                    let group_name = split.next().expect("Invalid format for FixedGroups");
                    let group_val = split.next().expect("Invalid format for FixedGroups");
                    fixed_groups.insert(group_name.to_string(), group_val.to_string());
                }
            }
            _ => panic!("Unrecognised key in StatTrigger attribute"),
        }
    }

    let id = id.expect("StatTrigger missing value for StatName");
    let field_groups = if let syn::Data::Struct(ref data_struct) = *body {
        data_struct
            .fields
            .iter()
            .filter(|f| {
                f.attrs
                    .iter()
                    .any(|a| a.path.is_ident("StatGroup") && is_attr_stat_id(a, &id))
            })
            .map(|f| f.clone().ident.expect("No identifier for field!"))
            .collect::<Vec<_>>()
    } else {
        vec![]
    };

    let bucket_field = if let syn::Data::Struct(ref data_struct) = *body {
        let bucket_by_fields = data_struct
            .fields
            .iter()
            .filter(|f| {
                f.attrs
                    .iter()
                    .any(|a| a.path.is_ident("BucketBy") && is_attr_stat_id(a, &id))
            })
            .map(|f| f.clone().ident.expect("No identifier for field!"))
            .collect::<Vec<_>>();

        if bucket_by_fields.len() > 1 {
            panic!("The BucketBy attribute can be added to at most one field");
        }

        bucket_by_fields.into_iter().next()
    } else {
        None
    };

    StatTriggerData {
        id,
        // If no condition is provided, default to always passing.
        condition_body: cond.unwrap_or_else(|| syn::parse_quote!(true)),
        action: action.expect("StatTrigger missing value for Action"),
        val: value.expect("StatTrigger missing value for Value or ValueFrom"),
        fixed_groups,
        field_groups,
        bucket_by: bucket_field,
    }
}
// LCOV_EXCL_STOP

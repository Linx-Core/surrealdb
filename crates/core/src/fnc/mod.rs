//! Executes functions from SQL. If there is an SQL function it will be defined in this module.

use crate::ctx::Context;
use crate::dbs::capabilities::ExperimentalTarget;
use crate::dbs::Options;
use crate::doc::CursorDoc;
use crate::err::Error;
use crate::idx::planner::executor::QueryExecutor;
use crate::sql::value::Value;
use crate::sql::Thing;
use reblessive::tree::Stk;
pub mod api;
pub mod args;
pub mod array;
pub mod bytes;
pub mod count;
pub mod crypto;
pub mod duration;
pub mod encoding;
pub mod file;
pub mod geo;
pub mod http;
pub mod math;
pub mod not;
pub mod object;
pub mod operate;
pub mod parse;
pub mod rand;
pub mod record;
pub mod script;
pub mod search;
pub mod session;
pub mod sleep;
pub mod string;
pub mod time;
pub mod r#type;
pub mod util;
pub mod value;
pub mod vector;

/// Attempts to run any function
pub async fn run(
	stk: &mut Stk,
	ctx: &Context,
	opt: &Options,
	doc: Option<&CursorDoc>,
	name: &str,
	args: Vec<Value>,
) -> Result<Value, Error> {
	if name.eq("sleep")
		|| name.eq("api::invoke")
		|| name.eq("array::all")
		|| name.eq("array::any")
		|| name.eq("array::every")
		|| name.eq("array::filter_index")
		|| name.eq("array::filter")
		|| name.eq("array::find_index")
		|| name.eq("array::find")
		|| name.eq("array::fold")
		|| name.eq("array::includes")
		|| name.eq("array::index_of")
		|| name.eq("array::map")
		|| name.eq("array::reduce")
		|| name.eq("array::some")
		|| name.eq("file::put")
		|| name.eq("file::put_if_not_exists")
		|| name.eq("file::get")
		|| name.eq("file::head")
		|| name.eq("file::delete")
		|| name.eq("file::exists")
		|| name.eq("file::copy")
		|| name.eq("file::copy_if_not_exists")
		|| name.eq("file::rename")
		|| name.eq("file::rename_if_not_exists")
		|| name.eq("file::list")
		|| name.eq("record::exists")
		|| name.eq("record::refs")
		|| name.eq("type::field")
		|| name.eq("type::fields")
		|| name.eq("value::diff")
		|| name.eq("value::patch")
		|| name.starts_with("http")
		|| name.starts_with("search")
		|| name.starts_with("crypto::argon2")
		|| name.starts_with("crypto::bcrypt")
		|| name.starts_with("crypto::pbkdf2")
		|| name.starts_with("crypto::scrypt")
	{
		stk.run(|stk| asynchronous(stk, ctx, opt, doc, name, args)).await
	} else {
		synchronous(ctx, doc, name, args)
	}
}

/// Each function is specified by its name (a string literal) followed by its path. The path
/// may be followed by one parenthesized argument, e.g. ctx, which is passed to the function
/// before the remainder of the arguments. The path may be followed by `.await` to signify that
/// it is `async`. Finally, the path may be prefixed by a parenthesized wrapper function e.g.
/// `cpu_intensive`.
macro_rules! dispatch {
    ($ctx: ident, $name: ident, $args: expr, $message: expr,
        $($(exp($exp_target: ident))? $function_name: literal =>
            $(($wrapper: tt))* $($function_path: ident)::+ $(($ctx_arg: expr))* $(.$await:tt)*,)+
    ) => {
        {
            match $name {
                $(
                    $function_name => {
                        $(
                            if !$ctx.get_capabilities().allows_experimental(&ExperimentalTarget::$exp_target) {
                                return Err($crate::err::Error::InvalidFunction {
                                    name: String::from($name),
                                    message: format!("Experimental feature {} is not enabled", ExperimentalTarget::$exp_target),
                                });
                            }
                        )?

                        let args = args::FromArgs::from_args($name, $args)?;
                        #[allow(clippy::redundant_closure_call)]
                        $($wrapper)*(|| $($function_path)::+($($ctx_arg,)* args))()$(.$await)*
                    },
                )+
                _ => {
                    Err($crate::err::Error::InvalidFunction{
                        name: String::from($name),
                        message: $message.to_string()
                    })
                }
            }
        }
    };
}

/// Attempts to run any synchronous function.
pub fn synchronous(
	ctx: &Context,
	doc: Option<&CursorDoc>,
	name: &str,
	args: Vec<Value>,
) -> Result<Value, Error> {
	dispatch!(
		ctx,
		name,
		args,
		"no such builtin function found",
		//
		"array::add" => array::add,
		"array::append" => array::append,
		"array::at" => array::at,
		"array::boolean_and" => array::boolean_and,
		"array::boolean_not" => array::boolean_not,
		"array::boolean_or" => array::boolean_or,
		"array::boolean_xor" => array::boolean_xor,
		"array::clump" => array::clump,
		"array::combine" => array::combine,
		"array::complement" => array::complement,
		"array::concat" => array::concat,
		"array::difference" => array::difference,
		"array::distinct" => array::distinct,
		"array::fill" => array::fill,
		"array::first" => array::first,
		"array::flatten" => array::flatten,
		"array::group" => array::group,
		"array::insert" => array::insert,
		"array::intersect" => array::intersect,
		"array::is_empty" => array::is_empty,
		"array::join" => array::join,
		"array::last" => array::last,
		"array::len" => array::len,
		"array::logical_and" => array::logical_and,
		"array::logical_or" => array::logical_or,
		"array::logical_xor" => array::logical_xor,
		"array::matches" => array::matches,
		"array::max" => array::max,
		"array::min" => array::min,
		"array::pop" => array::pop,
		"array::prepend" => array::prepend,
		"array::push" => array::push,
		"array::range" => array::range,
		"array::remove" => array::remove,
		"array::repeat" => array::repeat,
		"array::reverse" => array::reverse,
		"array::shuffle" => array::shuffle,
		"array::slice" => array::slice,
		"array::sort" => array::sort,
		"array::sort_natural" => array::sort_natural,
		"array::sort_lexical" => array::sort_lexical,
		"array::sort_natural_lexical" => array::sort_natural_lexical,
		"array::swap" => array::swap,
		"array::transpose" => array::transpose,
		"array::union" => array::union,
		"array::sort::asc" => array::sort::asc,
		"array::sort::desc" => array::sort::desc,
		"array::windows" => array::windows,
		//
		"bytes::len" => bytes::len,
		//
		"count" => count::count,
		//
		"crypto::blake3" => crypto::blake3,
		"crypto::md5" => crypto::md5,
		"crypto::sha1" => crypto::sha1,
		"crypto::sha256" => crypto::sha256,
		"crypto::sha512" => crypto::sha512,
		//
		"duration::days" => duration::days,
		"duration::hours" => duration::hours,
		"duration::micros" => duration::micros,
		"duration::millis" => duration::millis,
		"duration::mins" => duration::mins,
		"duration::nanos" => duration::nanos,
		"duration::secs" => duration::secs,
		"duration::weeks" => duration::weeks,
		"duration::years" => duration::years,
		"duration::from::days" => duration::from::days,
		"duration::from::hours" => duration::from::hours,
		"duration::from::micros" => duration::from::micros,
		"duration::from::millis" => duration::from::millis,
		"duration::from::mins" => duration::from::mins,
		"duration::from::nanos" => duration::from::nanos,
		"duration::from::secs" => duration::from::secs,
		"duration::from::weeks" => duration::from::weeks,
		//
		exp(Files) "file::bucket" => file::bucket,
		exp(Files) "file::key" => file::key,
		//
		"encoding::base64::decode" => encoding::base64::decode,
		"encoding::base64::encode" => encoding::base64::encode,
		"encoding::cbor::decode" => encoding::cbor::decode,
		"encoding::cbor::encode" => encoding::cbor::encode,
		//
		"geo::area" => geo::area,
		"geo::bearing" => geo::bearing,
		"geo::centroid" => geo::centroid,
		"geo::distance" => geo::distance,
		"geo::hash::decode" => geo::hash::decode,
		"geo::hash::encode" => geo::hash::encode,
		"geo::is::valid" => geo::is::valid,
		//
		"math::abs" => math::abs,
		"math::acos" => math::acos,
		"math::acot" => math::acot,
		"math::asin" => math::asin,
		"math::atan" => math::atan,
		"math::bottom" => math::bottom,
		"math::ceil" => math::ceil,
		"math::clamp" => math::clamp,
		"math::cos" => math::cos,
		"math::cot" => math::cot,
		"math::deg2rad" => math::deg2rad,
		"math::fixed" => math::fixed,
		"math::floor" => math::floor,
		"math::interquartile" => math::interquartile,
		"math::lerp" => math::lerp,
		"math::lerpangle" => math::lerpangle,
		"math::ln" => math::ln,
		"math::log" => math::log,
		"math::log10" => math::log10,
		"math::log2" => math::log2,
		"math::max" => math::max,
		"math::mean" => math::mean,
		"math::median" => math::median,
		"math::midhinge" => math::midhinge,
		"math::min" => math::min,
		"math::mode" => math::mode,
		"math::nearestrank" => math::nearestrank,
		"math::percentile" => math::percentile,
		"math::pow" => math::pow,
		"math::product" => math::product,
		"math::rad2deg" => math::rad2deg,
		"math::round" => math::round,
		"math::sign" => math::sign,
		"math::sin" => math::sin,
		"math::spread" => math::spread,
		"math::sqrt" => math::sqrt,
		"math::stddev" => math::stddev,
		"math::sum" => math::sum,
		"math::tan" => math::tan,
		"math::top" => math::top,
		"math::trimean" => math::trimean,
		"math::variance" => math::variance,
		//
		"meta::id" => record::id,
		"meta::tb" => record::tb,
		//
		"not" => not::not,
		//
		"object::entries" => object::entries,
		"object::from_entries" => object::from_entries,
		"object::is_empty" => object::is_empty,
		"object::keys" => object::keys,
		"object::len" => object::len,
		"object::values" => object::values,
		//
		"parse::email::host" => parse::email::host,
		"parse::email::user" => parse::email::user,
		"parse::url::domain" => parse::url::domain,
		"parse::url::fragment" => parse::url::fragment,
		"parse::url::host" => parse::url::host,
		"parse::url::path" => parse::url::path,
		"parse::url::port" => parse::url::port,
		"parse::url::query" => parse::url::query,
		"parse::url::scheme" => parse::url::scheme,
		//
		"rand" => rand::rand,
		"rand::bool" => rand::bool,
		"rand::enum" => rand::r#enum,
		"rand::float" => rand::float,
		"rand::guid" => rand::guid,
		"rand::int" => rand::int,
		"rand::string" => rand::string,
		"rand::time" => rand::time,
		"rand::ulid" => rand::ulid,
		"rand::uuid::v4" => rand::uuid::v4,
		"rand::uuid::v7" => rand::uuid::v7,
		"rand::uuid" => rand::uuid,
		//
		"record::id" => record::id,
		"record::table" => record::tb,
		"record::tb" => record::tb,
		//
		"session::ac" => session::ac(ctx),
		"session::db" => session::db(ctx),
		"session::id" => session::id(ctx),
		"session::ip" => session::ip(ctx),
		"session::ns" => session::ns(ctx),
		"session::origin" => session::origin(ctx),
		"session::rd" => session::rd(ctx),
		"session::token" => session::token(ctx),
		//
		"string::concat" => string::concat,
		"string::contains" => string::contains,
		"string::ends_with" => string::ends_with,
		"string::join" => string::join,
		"string::len" => string::len,
		"string::lowercase" => string::lowercase,
		"string::matches" => string::matches,
		"string::repeat" => string::repeat,
		"string::replace" => string::replace,
		"string::reverse" => string::reverse,
		"string::slice" => string::slice,
		"string::slug" => string::slug,
		"string::split" => string::split,
		"string::starts_with" => string::starts_with,
		"string::trim" => string::trim,
		"string::uppercase" => string::uppercase,
		"string::words" => string::words,
		//
		"string::distance::damerau_levenshtein" => string::distance::damerau_levenshtein,
		"string::distance::hamming" => string::distance::hamming,
		"string::distance::levenshtein" => string::distance::levenshtein,
		"string::distance::normalized_damerau_levenshtein" => string::distance::normalized_damerau_levenshtein,
		"string::distance::normalized_levenshtein" => string::distance::normalized_levenshtein,
		"string::distance::osa_distance" => string::distance::osa_distance,
		//
		"string::html::encode" => string::html::encode,
		"string::html::sanitize" => string::html::sanitize,
		"string::is::alphanum" => string::is::alphanum,
		"string::is::alpha" => string::is::alpha,
		"string::is::ascii" => string::is::ascii,
		"string::is::datetime" => string::is::datetime,
		"string::is::domain" => string::is::domain,
		"string::is::email" => string::is::email,
		"string::is::hexadecimal" => string::is::hexadecimal,
		"string::is::ip" => string::is::ip,
		"string::is::ipv4" => string::is::ipv4,
		"string::is::ipv6" => string::is::ipv6,
		"string::is::latitude" => string::is::latitude,
		"string::is::longitude" => string::is::longitude,
		"string::is::numeric" => string::is::numeric,
		"string::is::semver" => string::is::semver,
		"string::is::url" => string::is::url,
		"string::is::ulid" => string::is::ulid,
		"string::is::uuid" => string::is::uuid,
		"string::is::record" => string::is::record,
		//
		"string::similarity::fuzzy" => string::similarity::fuzzy,
		"string::similarity::jaro" => string::similarity::jaro,
		"string::similarity::jaro_winkler" => string::similarity::jaro_winkler,
		"string::similarity::smithwaterman" => string::similarity::smithwaterman,
		"string::similarity::sorensen_dice" => string::similarity::sorensen_dice,
		//
		"string::semver::compare" => string::semver::compare,
		"string::semver::major" => string::semver::major,
		"string::semver::minor" => string::semver::minor,
		"string::semver::patch" => string::semver::patch,
		"string::semver::inc::major" => string::semver::inc::major,
		"string::semver::inc::minor" => string::semver::inc::minor,
		"string::semver::inc::patch" => string::semver::inc::patch,
		"string::semver::set::major" => string::semver::set::major,
		"string::semver::set::minor" => string::semver::set::minor,
		"string::semver::set::patch" => string::semver::set::patch,
		//
		"time::ceil" => time::ceil,
		"time::day" => time::day,
		"time::floor" => time::floor,
		"time::format" => time::format,
		"time::group" => time::group,
		"time::hour" => time::hour,
		"time::max" => time::max,
		"time::min" => time::min,
		"time::minute" => time::minute,
		"time::month" => time::month,
		"time::nano" => time::nano,
		"time::micros" => time::micros,
		"time::millis" => time::millis,
		"time::now" => time::now,
		"time::round" => time::round,
		"time::second" => time::second,
		"time::timezone" => time::timezone,
		"time::unix" => time::unix,
		"time::wday" => time::wday,
		"time::week" => time::week,
		"time::yday" => time::yday,
		"time::year" => time::year,
		"time::from::nanos" => time::from::nanos,
		"time::from::micros" => time::from::micros,
		"time::from::millis" => time::from::millis,
		"time::from::secs" => time::from::secs,
		"time::from::ulid" => time::from::ulid,
		"time::from::unix" => time::from::unix,
		"time::from::uuid" => time::from::uuid,
		"time::is::leap_year" => time::is::leap_year,
		//
		"type::array" => r#type::array,
		"type::bool" => r#type::bool,
		"type::bytes" => r#type::bytes,
		"type::datetime" => r#type::datetime,
		"type::decimal" => r#type::decimal,
		"type::duration" => r#type::duration,
		exp(Files) "type::file" => r#type::file,
		"type::float" => r#type::float,
		"type::geometry" => r#type::geometry,
		"type::int" => r#type::int,
		"type::number" => r#type::number,
		"type::point" => r#type::point,
		"type::range" => r#type::range,
		"type::record" => r#type::record,
		"type::string" => r#type::string,
		"type::string_lossy" => r#type::string_lossy,
		"type::table" => r#type::table,
		"type::thing" => r#type::thing,
		"type::uuid" => r#type::uuid,
		"type::is::array" => r#type::is::array,
		"type::is::bool" => r#type::is::bool,
		"type::is::bytes" => r#type::is::bytes,
		"type::is::collection" => r#type::is::collection,
		"type::is::datetime" => r#type::is::datetime,
		"type::is::decimal" => r#type::is::decimal,
		"type::is::duration" => r#type::is::duration,
		"type::is::float" => r#type::is::float,
		"type::is::geometry" => r#type::is::geometry,
		"type::is::int" => r#type::is::int,
		"type::is::line" => r#type::is::line,
		"type::is::none" => r#type::is::none,
		"type::is::null" => r#type::is::null,
		"type::is::multiline" => r#type::is::multiline,
		"type::is::multipoint" => r#type::is::multipoint,
		"type::is::multipolygon" => r#type::is::multipolygon,
		"type::is::number" => r#type::is::number,
		"type::is::object" => r#type::is::object,
		"type::is::point" => r#type::is::point,
		"type::is::polygon" => r#type::is::polygon,
		"type::is::range" => r#type::is::range,
		"type::is::record" => r#type::is::record,
		"type::is::string" => r#type::is::string,
		"type::is::uuid" => r#type::is::uuid,
		//
		"vector::add" => vector::add,
		"vector::angle" => vector::angle,
		"vector::cross" => vector::cross,
		"vector::dot" => vector::dot,
		"vector::divide" => vector::divide,
		"vector::magnitude" => vector::magnitude,
		"vector::multiply" => vector::multiply,
		"vector::normalize" => vector::normalize,
		"vector::project" => vector::project,
		"vector::scale" => vector::scale,
		"vector::subtract" => vector::subtract,
		"vector::distance::chebyshev" => vector::distance::chebyshev,
		"vector::distance::euclidean" => vector::distance::euclidean,
		"vector::distance::hamming" => vector::distance::hamming,
		"vector::distance::knn" => vector::distance::knn((ctx, doc)),
		"vector::distance::mahalanobis" => vector::distance::mahalanobis,
		"vector::distance::manhattan" => vector::distance::manhattan,
		"vector::distance::minkowski" => vector::distance::minkowski,
		"vector::similarity::cosine" => vector::similarity::cosine,
		"vector::similarity::jaccard" => vector::similarity::jaccard,
		"vector::similarity::pearson" => vector::similarity::pearson,
		"vector::similarity::spearman" => vector::similarity::spearman,
	)
}

/// Attempts to run any asynchronous function.
pub async fn asynchronous(
	stk: &mut Stk,
	ctx: &Context,
	opt: &Options,
	doc: Option<&CursorDoc>,
	name: &str,
	args: Vec<Value>,
) -> Result<Value, Error> {
	// Wrappers return a function as opposed to a value so that the dispatch! method can always
	// perform a function call.
	#[cfg(not(target_family = "wasm"))]
	fn cpu_intensive<R: Send + 'static>(
		function: impl FnOnce() -> R + Send + 'static,
	) -> impl FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = R> + Send>> {
		|| Box::pin(crate::exe::spawn(function))
	}

	#[cfg(target_family = "wasm")]
	fn cpu_intensive<R: Send + 'static>(
		function: impl FnOnce() -> R + Send + 'static,
	) -> impl FnOnce() -> std::future::Ready<R> {
		|| std::future::ready(function())
	}

	dispatch!(
		ctx,
		name,
		args,
		"no such builtin function found",
		//
		exp(DefineApi) "api::invoke" => api::invoke((stk, ctx, opt)).await,
		//
		"array::all" => array::all((stk, ctx, Some(opt), doc)).await,
		"array::any" => array::any((stk, ctx, Some(opt), doc)).await,
		"array::every" => array::all((stk, ctx, Some(opt), doc)).await,
		"array::filter" => array::filter((stk, ctx, Some(opt), doc)).await,
		"array::filter_index" => array::filter_index((stk, ctx, Some(opt), doc)).await,
		"array::find" => array::find((stk, ctx, Some(opt), doc)).await,
		"array::find_index" => array::find_index((stk, ctx, Some(opt), doc)).await,
		"array::fold" => array::fold((stk, ctx, Some(opt), doc)).await,
		"array::includes" => array::any((stk, ctx, Some(opt), doc)).await,
		"array::index_of" => array::find_index((stk, ctx, Some(opt), doc)).await,
		"array::map" => array::map((stk, ctx, Some(opt), doc)).await,
		"array::reduce" => array::reduce((stk, ctx, Some(opt), doc)).await,
		"array::some" => array::any((stk, ctx, Some(opt), doc)).await,
		//
		"crypto::argon2::compare" => (cpu_intensive) crypto::argon2::cmp.await,
		"crypto::argon2::generate" => (cpu_intensive) crypto::argon2::gen.await,
		"crypto::bcrypt::compare" => (cpu_intensive) crypto::bcrypt::cmp.await,
		"crypto::bcrypt::generate" => (cpu_intensive) crypto::bcrypt::gen.await,
		"crypto::pbkdf2::compare" => (cpu_intensive) crypto::pbkdf2::cmp.await,
		"crypto::pbkdf2::generate" => (cpu_intensive) crypto::pbkdf2::gen.await,
		"crypto::scrypt::compare" => (cpu_intensive) crypto::scrypt::cmp.await,
		"crypto::scrypt::generate" => (cpu_intensive) crypto::scrypt::gen.await,
		//
		exp(Files) "file::put" => file::put((stk, ctx, opt, doc)).await,
		exp(Files) "file::put_if_not_exists" => file::put_if_not_exists((stk, ctx, opt, doc)).await,
		exp(Files) "file::get" => file::get((stk, ctx, opt, doc)).await,
		exp(Files) "file::head" => file::head((stk, ctx, opt, doc)).await,
		exp(Files) "file::delete" => file::delete((stk, ctx, opt, doc)).await,
		exp(Files) "file::copy" => file::copy((stk, ctx, opt, doc)).await,
		exp(Files) "file::copy_if_not_exists" => file::copy_if_not_exists((stk, ctx, opt, doc)).await,
		exp(Files) "file::rename" => file::rename((stk, ctx, opt, doc)).await,
		exp(Files) "file::rename_if_not_exists" => file::rename_if_not_exists((stk, ctx, opt, doc)).await,
		exp(Files) "file::exists" => file::exists((stk, ctx, opt, doc)).await,
		exp(Files) "file::list" => file::list((stk, ctx, opt, doc)).await,
		//
		"http::head" => http::head(ctx).await,
		"http::get" => http::get(ctx).await,
		"http::put" => http::put(ctx).await,
		"http::post" =>  http::post(ctx).await,
		"http::patch" => http::patch(ctx).await,
		"http::delete" => http::delete(ctx).await,
		//
		"record::exists" => record::exists((stk, ctx, Some(opt), doc)).await,
		"record::refs" => record::refs((stk, ctx, opt, doc)).await,
		//
		"search::analyze" => search::analyze((stk, ctx, Some(opt))).await,
		"search::score" => search::score((ctx, doc)).await,
		"search::highlight" => search::highlight((ctx, doc)).await,
		"search::offsets" => search::offsets((ctx, doc)).await,
		//
		"sleep" => sleep::sleep(ctx).await,
		//
		"type::field" => r#type::field((stk, ctx, Some(opt), doc)).await,
		"type::fields" => r#type::fields((stk, ctx, Some(opt), doc)).await,
		//
		"value::diff" => value::diff((stk, ctx, Some(opt), doc)).await,
		"value::patch" => value::patch((stk, ctx, Some(opt), doc)).await,
	)
}

/// Attempts to run any synchronous function.
pub async fn idiom(
	stk: &mut Stk,
	ctx: &Context,
	opt: &Options,
	doc: Option<&CursorDoc>,
	value: Value,
	name: &str,
	args: Vec<Value>,
) -> Result<Value, Error> {
	let args = [vec![value.clone()], args].concat();
	let specific = match value {
		Value::Array(_) => {
			dispatch!(
				ctx,
				name,
				args.clone(),
				"no such method found for the array type",
				//
				"add" => array::add,
				"all" => array::all((stk, ctx, Some(opt), doc)).await,
				"any" => array::any((stk, ctx, Some(opt), doc)).await,
				"append" => array::append,
				"at" => array::at,
				"boolean_and" => array::boolean_and,
				"boolean_not" => array::boolean_not,
				"boolean_or" => array::boolean_or,
				"boolean_xor" => array::boolean_xor,
				"clump" => array::clump,
				"combine" => array::combine,
				"complement" => array::complement,
				"concat" => array::concat,
				"difference" => array::difference,
				"distinct" => array::distinct,
				"every" => array::all((stk, ctx, Some(opt), doc)).await,
				"fill" => array::fill,
				"filter" => array::filter((stk, ctx, Some(opt), doc)).await,
				"filter_index" => array::filter_index((stk, ctx, Some(opt), doc)).await,
				"find" => array::find((stk, ctx, Some(opt), doc)).await,
				"find_index" => array::find_index((stk, ctx, Some(opt), doc)).await,
				"first" => array::first,
				"fold" => array::fold((stk, ctx, Some(opt), doc)).await,
				"flatten" => array::flatten,
				"group" => array::group,
				"includes" => array::any((stk, ctx, Some(opt), doc)).await,
				"index_of" => array::find_index((stk, ctx, Some(opt), doc)).await,
				"insert" => array::insert,
				"intersect" => array::intersect,
				"is_empty" => array::is_empty,
				"join" => array::join,
				"last" => array::last,
				"len" => array::len,
				"logical_and" => array::logical_and,
				"logical_or" => array::logical_or,
				"logical_xor" => array::logical_xor,
				"matches" => array::matches,
				"map" => array::map((stk, ctx, Some(opt), doc)).await,
				"max" => array::max,
				"min" => array::min,
				"pop" => array::pop,
				"prepend" => array::prepend,
				"push" => array::push,
				"reduce" => array::reduce((stk, ctx, Some(opt), doc)).await,
				"remove" => array::remove,
				"reverse" => array::reverse,
				"shuffle" => array::shuffle,
				"slice" => array::slice,
				"some" => array::any((stk, ctx, Some(opt), doc)).await,
				"sort" => array::sort,
				"sort_natural" => array::sort_natural,
				"sort_lexical" => array::sort_lexical,
				"sort_natural_lexical" => array::sort_natural_lexical,
				"swap" => array::swap,
				"transpose" => array::transpose,
				"union" => array::union,
				"sort_asc" => array::sort::asc,
				"sort_desc" => array::sort::desc,
				"windows" => array::windows,
				//
				"vector_add" => vector::add,
				"vector_angle" => vector::angle,
				"vector_cross" => vector::cross,
				"vector_dot" => vector::dot,
				"vector_divide" => vector::divide,
				"vector_magnitude" => vector::magnitude,
				"vector_multiply" => vector::multiply,
				"vector_normalize" => vector::normalize,
				"vector_project" => vector::project,
				"vector_scale" => vector::scale,
				"vector_subtract" => vector::subtract,
				"vector_distance_chebyshev" => vector::distance::chebyshev,
				"vector_distance_euclidean" => vector::distance::euclidean,
				"vector_distance_hamming" => vector::distance::hamming,
				"vector_distance_knn" => vector::distance::knn((ctx, doc)),
				"vector_distance_mahalanobis" => vector::distance::mahalanobis,
				"vector_distance_manhattan" => vector::distance::manhattan,
				"vector_distance_minkowski" => vector::distance::minkowski,
				"vector_similarity_cosine" => vector::similarity::cosine,
				"vector_similarity_jaccard" => vector::similarity::jaccard,
				"vector_similarity_pearson" => vector::similarity::pearson,
				"vector_similarity_spearman" => vector::similarity::spearman,
			)
		}
		Value::Bytes(_) => {
			dispatch!(
				ctx,
				name,
				args.clone(),
				"no such method found for the bytes type",
				//
				"len" => bytes::len,
			)
		}
		Value::Duration(_) => {
			dispatch!(
				ctx,
				name,
				args.clone(),
				"no such method found for the duration type",
				//
				"days" => duration::days,
				"hours" => duration::hours,
				"micros" => duration::micros,
				"millis" => duration::millis,
				"mins" => duration::mins,
				"nanos" => duration::nanos,
				"secs" => duration::secs,
				"weeks" => duration::weeks,
				"years" => duration::years,
			)
		}
		Value::Geometry(_) => {
			dispatch!(
				ctx,
				name,
				args.clone(),
				"no such method found for the geometry type",
				//
				"area" => geo::area,
				"bearing" => geo::bearing,
				"centroid" => geo::centroid,
				"distance" => geo::distance,
				"hash_decode" => geo::hash::decode,
				"hash_encode" => geo::hash::encode,
				"is_valid" => geo::is::valid,
			)
		}
		Value::Thing(_) => {
			dispatch!(
				ctx,
				name,
				args.clone(),
				"no such method found for the record type",
				//
				"exists" => record::exists((stk, ctx, Some(opt), doc)).await,
				"id" => record::id,
				"table" => record::tb,
				"tb" => record::tb,
				"refs" => record::refs((stk, ctx, opt, doc)).await,
			)
		}
		Value::Object(_) => {
			dispatch!(
				ctx,
				name,
				args.clone(),
				"no such method found for the object type",
				//
				"entries" => object::entries,
				"is_empty" => object::is_empty,
				"keys" => object::keys,
				"len" => object::len,
				"values" => object::values,
			)
		}
		Value::Number(_) => {
			dispatch!(
				ctx,
				name,
				args.clone(),
				"no such method found for the number type",
				//
				"abs" => math::abs,
				"acos" => math::acos,
				"acot" => math::acot,
				"asin" => math::asin,
				"atan" => math::atan,
				"ceil" => math::ceil,
				"cos" => math::cos,
				"cot" => math::cot,
				"deg2rad" => math::deg2rad,
				"floor" => math::floor,
				"ln" => math::ln,
				"log" => math::log,
				"log10" => math::log10,
				"log2" => math::log2,
				"rad2deg" => math::rad2deg,
				"round" => math::round,
				"sign" => math::sign,
				"sin" => math::sin,
				"tan" => math::tan,
			)
		}
		Value::Strand(_) => {
			dispatch!(
				ctx,
				name,
				args.clone(),
				"no such method found for the string type",
				//
				"concat" => string::concat,
				"contains" => string::contains,
				"ends_with" => string::ends_with,
				"join" => string::join,
				"len" => string::len,
				"lowercase" => string::lowercase,
				"matches" => string::matches,
				"repeat" => string::repeat,
				"replace" => string::replace,
				"reverse" => string::reverse,
				"slice" => string::slice,
				"slug" => string::slug,
				"split" => string::split,
				"starts_with" => string::starts_with,
				"trim" => string::trim,
				"uppercase" => string::uppercase,
				"words" => string::words,
				"distance_damerau_levenshtein" => string::distance::damerau_levenshtein,
				"distance_hamming" => string::distance::hamming,
				"distance_levenshtein" => string::distance::levenshtein,
				"distance_normalized_damerau_levenshtein" => string::distance::normalized_damerau_levenshtein,
				"distance_normalized_levenshtein" => string::distance::normalized_levenshtein,
				"html_encode" => string::html::encode,
				"html_sanitize" => string::html::sanitize,
				"is_alphanum" => string::is::alphanum,
				"is_alpha" => string::is::alpha,
				"is_ascii" => string::is::ascii,
				"is_datetime" => string::is::datetime,
				"is_domain" => string::is::domain,
				"is_email" => string::is::email,
				"is_hexadecimal" => string::is::hexadecimal,
				"is_ip" => string::is::ip,
				"is_ipv4" => string::is::ipv4,
				"is_ipv6" => string::is::ipv6,
				"is_latitude" => string::is::latitude,
				"is_longitude" => string::is::longitude,
				"is_numeric" => string::is::numeric,
				"is_semver" => string::is::semver,
				"is_url" => string::is::url,
				"is_ulid" => string::is::ulid,
				"is_uuid" => string::is::uuid,
				"is_record" => string::is::record,
				"similarity_fuzzy" => string::similarity::fuzzy,
				"similarity_jaro" => string::similarity::jaro,
				"similarity_jaro_winkler" => string::similarity::jaro_winkler,
				"similarity_smithwaterman" => string::similarity::smithwaterman,
				"similarity_sorensen_dice" => string::similarity::sorensen_dice,
				"semver_compare" => string::semver::compare,
				"semver_major" => string::semver::major,
				"semver_minor" => string::semver::minor,
				"semver_patch" => string::semver::patch,
				"semver_inc_major" => string::semver::inc::major,
				"semver_inc_minor" => string::semver::inc::minor,
				"semver_inc_patch" => string::semver::inc::patch,
				"semver_set_major" => string::semver::set::major,
				"semver_set_minor" => string::semver::set::minor,
				"semver_set_patch" => string::semver::set::patch,
			)
		}
		Value::Datetime(_) => {
			dispatch!(
				ctx,
				name,
				args.clone(),
				"no such method found for the datetime type",
				//
				"ceil" => time::ceil,
				"day" => time::day,
				"floor" => time::floor,
				"format" => time::format,
				"group" => time::group,
				"hour" => time::hour,
				"is_leap_year" => time::is::leap_year,
				"micros" => time::micros,
				"millis" => time::millis,
				"minute" => time::minute,
				"month" => time::month,
				"nano" => time::nano,
				"round" => time::round,
				"second" => time::second,
				"unix" => time::unix,
				"wday" => time::wday,
				"week" => time::week,
				"yday" => time::yday,
				"year" => time::year,
			)
		}
		Value::File(_) => {
			dispatch!(
				ctx,
				name,
				args.clone(),
				"no such method found for the file type",
				//
				exp(Files) "bucket" => file::bucket,
				exp(Files) "key" => file::key,
				//
				exp(Files) "put" => file::put((stk, ctx, opt, doc)).await,
				exp(Files) "put_if_not_exists" => file::put_if_not_exists((stk, ctx, opt, doc)).await,
				exp(Files) "get" => file::get((stk, ctx, opt, doc)).await,
				exp(Files) "head" => file::head((stk, ctx, opt, doc)).await,
				exp(Files) "delete" => file::delete((stk, ctx, opt, doc)).await,
				exp(Files) "copy" => file::copy((stk, ctx, opt, doc)).await,
				exp(Files) "copy_if_not_exists" => file::copy_if_not_exists((stk, ctx, opt, doc)).await,
				exp(Files) "rename" => file::rename((stk, ctx, opt, doc)).await,
				exp(Files) "rename_if_not_exists" => file::rename_if_not_exists((stk, ctx, opt, doc)).await,
				exp(Files) "exists" => file::exists((stk, ctx, opt, doc)).await,
			)
		}
		_ => Err(Error::InvalidFunction {
			name: "".into(),
			message: "".into(),
		}),
	};

	match specific {
		Err(Error::InvalidFunction {
			..
		}) => {
			let message = format!("no such method found for the {} type", value.kindof());
			dispatch!(
				ctx,
				name,
				args,
				message,
				//
				"is_array" => r#type::is::array,
				"is_bool" => r#type::is::bool,
				"is_bytes" => r#type::is::bytes,
				"is_collection" => r#type::is::collection,
				"is_datetime" => r#type::is::datetime,
				"is_decimal" => r#type::is::decimal,
				"is_duration" => r#type::is::duration,
				"is_float" => r#type::is::float,
				"is_geometry" => r#type::is::geometry,
				"is_int" => r#type::is::int,
				"is_line" => r#type::is::line,
				"is_none" => r#type::is::none,
				"is_null" => r#type::is::null,
				"is_multiline" => r#type::is::multiline,
				"is_multipoint" => r#type::is::multipoint,
				"is_multipolygon" => r#type::is::multipolygon,
				"is_number" => r#type::is::number,
				"is_object" => r#type::is::object,
				"is_point" => r#type::is::point,
				"is_polygon" => r#type::is::polygon,
				"is_range" => r#type::is::range,
				"is_record" => r#type::is::record,
				"is_string" => r#type::is::string,
				"is_uuid" => r#type::is::uuid,
				//
				"to_array" => r#type::array,
				"to_bool" => r#type::bool,
				"to_bytes" => r#type::bytes,
				"to_datetime" => r#type::datetime,
				"to_decimal" => r#type::decimal,
				"to_duration" => r#type::duration,
				"to_float" => r#type::float,
				"to_geometry" => r#type::geometry,
				"to_int" => r#type::int,
				"to_number" => r#type::number,
				"to_point" => r#type::point,
				"to_range" => r#type::range,
				"to_record" => r#type::record,
				"to_string" => r#type::string,
				"to_string_lossy" => r#type::string_lossy,
				"to_uuid" => r#type::uuid,
				//
				"chain" => value::chain((stk, ctx, Some(opt), doc)).await,
				"diff" => value::diff((stk, ctx, Some(opt), doc)).await,
				"patch" => value::patch((stk, ctx, Some(opt), doc)).await,
				//
				"repeat" => array::repeat,
			)
		}
		v => v,
	}
}

fn get_execution_context<'a>(
	ctx: &'a Context,
	doc: Option<&'a CursorDoc>,
) -> Option<(&'a QueryExecutor, &'a CursorDoc, &'a Thing)> {
	if let Some(doc) = doc {
		if let Some(thg) = &doc.rid {
			if let Some(pla) = ctx.get_query_planner() {
				if let Some(exe) = pla.get_query_executor(&thg.tb) {
					return Some((exe, doc, thg));
				}
			}
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use regex::Regex;

	use crate::dbs::Capabilities;
	use crate::{
		dbs::capabilities::ExperimentalTarget,
		sql::{statements::OutputStatement, Function, Query, Statement, Value},
	};

	#[tokio::test]
	async fn implementations_are_present() {
		// Accumulate and display all problems at once to avoid a test -> fix -> test -> fix cycle.
		let mut problems = Vec::new();

		// Read the source code of this file
		let fnc_mod = include_str!("mod.rs");

		// Patch out idiom methods
		let re = Regex::new(r"(?ms)pub async fn idiom\(.*}").unwrap();
		let fnc_no_idiom = re.replace(fnc_mod, "");

		let exp_regex = Regex::new(r"exp\(.*\) ").unwrap();

		for line in fnc_no_idiom.lines() {
			let line = line.trim();
			let line = if line.starts_with("exp") {
				&exp_regex.replace(line, "")
			} else {
				line
			};

			if !(line.contains("=>") && (line.starts_with('"') || line.ends_with(','))) {
				// This line does not define a function name.
				continue;
			}

			let (quote, _) = line.split_once("=>").unwrap();
			let name = quote.trim().trim_matches('"');

			let res = crate::syn::parse_with_capabilities(
				&format!("RETURN {}()", name),
				&Capabilities::all().with_experimental(ExperimentalTarget::DefineApi.into()),
			);

			if let Ok(Query(mut x)) = res {
				match x.0.pop() {
					Some(Statement::Output(OutputStatement {
						what: Value::Function(x),
						..
					})) => match *x {
						Function::Normal(parsed_name, _) => {
							if parsed_name != name {
								problems
									.push(format!("function `{name}` parsed as `{parsed_name}`"));
							}
						}
						_ => {
							problems.push(format!("couldn't parse {name} function"));
						}
					},
					_ => {
						problems.push(format!("couldn't parse {name} function"));
					}
				}
			} else {
				problems.push(format!("couldn't parse {name} function"));
			}

			#[cfg(all(feature = "scripting", feature = "kv-mem"))]
			{
				use crate::sql::Value;

				let name = name.replace("::", ".");
				let sql =
					format!("RETURN function() {{ return typeof surrealdb.functions.{name}; }}");
				let dbs = crate::kvs::Datastore::new("memory")
					.await
					.unwrap()
					.with_capabilities(Capabilities::all());
				let ses = crate::dbs::Session::owner().with_ns("test").with_db("test");
				let res = &mut dbs.execute(&sql, &ses, None).await.unwrap();
				let tmp = res.remove(0).result.unwrap();
				if tmp == Value::from("object") {
					// Assume this function is superseded by a module of the same name.
				} else if tmp != Value::from("function") {
					problems.push(format!("function {name} not exported to JavaScript: {tmp:?}"));
				}
			}
		}

		if !problems.is_empty() {
			eprintln!("Functions not fully implemented:");
			for problem in problems {
				eprintln!(" - {problem}");
			}
			panic!("ensure functions can be parsed in core/src/sql/function.rs and are exported to JS in core/src/fnc/script/modules/surrealdb");
		}
	}
}

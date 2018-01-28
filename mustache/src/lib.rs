#![allow(unused_variables)]
#[macro_use] extern crate serde_derive;

extern crate serde_json;
use serde_json::Value;

extern crate stache;
use stache::{ Template, TemplateEngine, TemplateCompiler, Partials };
use stache::testing::{ Pool };
use stache::rule::Rule;
use stache::error::{ RenderingError, CompilingError };

use std::collections::HashMap;

mod toolkit;
use self::toolkit::*;

#[derive(Deserialize, PartialEq, Debug, Clone)]
pub enum Mustache {
    Interpolation(String),
    EscapedInterpolation(String),
    Section(String),
    InvertedSection(String),
    Close(String),
    Partial(String),
    Comment(String),
    Default(String)
}

pub type Test = Pool<Mustache, Value, String>;

impl Default for Mustache {
    fn default() -> Self {
        Mustache::Default(String::default())
    }
}

impl Rule for Mustache {
    fn is_dotted(&self) -> bool {
        use self::Mustache::*;

        match *self {
            Interpolation(ref key) if key.contains(".") => true,
            EscapedInterpolation(ref key) if key.contains(".") => true,
            Section(ref key) if key.contains(".") => true,
            InvertedSection(ref key) if key.contains(".") => true,
            _ => false
        }
    }
}

impl TemplateCompiler for Mustache {
    fn compiles_template(input: String) -> Result<Template<Mustache>, CompilingError> {
        match Self::compiles(&include_str!("../Mustache.toml"), Some(input), None) {
            Ok((tmpl, _)) => Ok(tmpl),
            Err(err) => Err(err)
        }
    }

    fn compiles_partial(partials_input: HashMap<String, String>) -> Result<Partials<Mustache>, CompilingError> {
        match Self::compiles(&include_str!("../Mustache.toml"), None, Some(partials_input)) {
            Ok((_, partials)) => Ok(partials),
            Err(err) => Err(err)
        }
    }

    fn compiles_all(input: String, partials_input: HashMap<String, String>) -> Result<(Template<Mustache>, Partials<Mustache>), CompilingError> {
        Self::compiles(&include_str!("../Mustache.toml"), Some(input), Some(partials_input))
    }
}

impl TemplateEngine<Mustache, Value, String> for Mustache {
    fn render(template: Template<Mustache>, partials: Partials<Mustache>, contexts: Vec<Value>) -> Result<String, RenderingError> {
        let mut writter = Writter::new();
        let mut template = template.clone();
        let global = contexts.clone();

        while let Some(ref rule) = template.next() {
            let mut context_stack = global.iter().rev();

            while let Some(context) = context_stack.next() {
                use self::Mustache::*;

                match *rule {
                    Interpolation(ref key) => {
                        let key = match key.as_ref() {
                            "." => String::default(),
                            _ => key.clone()
                        };

                        if let Some(write) = interpolate(&key, context) {
                            writter.write(&write);
                        }
                    },
                    Section(ref key) => {
                        let close = Mustache::Close(key.clone());

                        if let Some(section) = template.split_until(&close) {
                            for new_context in interpolate_section(&key, &context, &global) {
                                match Mustache::render(section.clone(), partials.clone(), new_context) {
                                    Ok(write) => writter.write(&write),
                                    Err(error) => return Err(error)
                                }
                            }
                        } else {
                            return Err(RenderingError::InvalidStatement(
                                String::from("Incomplete template")
                            ));
                        }
                    },
                    InvertedSection(ref key) => {
                        let close = Mustache::Close(key.clone());

                        if let Some(section) = template.split_until(&close) {
                            for new_context in interpolate_inverted(&key, &context, &global) {
                                match Mustache::render(section.clone(), partials.clone(), new_context) {
                                    Ok(write) => writter.write(&write),
                                    Err(error) => return Err(error)
                                }
                            }
                        } else {
                            return Err(RenderingError::InvalidStatement(
                                String::from("Incomplete template")
                            ));
                        }
                    },
                    Close(_) => {},
                    Partial(ref key) => {
                        if let Some(template) = partials.get(key) {
                            let mut new_contexts = contexts.clone();

                            if let Some(context) = contexts.last() {
                                new_contexts = vec![context.clone()];
                            }

                            match Mustache::render(template.clone(), partials.clone(), new_contexts) {
                                Ok(write) => writter.write(&write),
                                Err(error) => return Err(error)
                            }
                        }
                    },
                    Comment(_) => {},
                    Default(ref value) => {
                        writter.write(&value);
                    }
                }

                if writter.is_written || rule.is_dotted() {
                    writter.reset();
                    break;
                }
            }
        }

        Ok(writter.buffer)
    }
}

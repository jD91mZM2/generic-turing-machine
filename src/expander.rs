use crate::{
    parser::{Types, Node},
    types::{Move, Type, Root, TypedNode},
    FINISH_STATE
};

use rowan::TextRange;
use std::collections::HashMap;

#[derive(Debug, Fail, PartialEq, Eq)]
pub enum GenerateError {
    #[fail(display = "invalid number of generic arguments: expected {} found {}", _0, _1)]
    IllegalArguments(usize, usize),
    #[fail(display = "can't match generic arguments on a generic type")]
    MatchGenerics,
    #[fail(display = "there must only be exactly one start assignment")]
    MultipleStart,
    #[fail(display = "unknown variable or function {:?}", _0)]
    Unknown(String)
}

type Result<T> = std::result::Result<T, (Option<TextRange>, GenerateError)>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FnSignature {
    /// The new matching state, serialized like state<arg0><arg1<sub_arg0>>
    pub match_state: String,
    /// Slicing the serialized name to this byte index will give you the original name
    pub original_len: usize,
    /// The input on the tape to match
    pub match_input: Option<u8>,
}
#[derive(Debug, Clone)]
pub struct FnBody {
    pub span: TextRange,
    pub do_write: Option<u8>,
    pub do_state: String,
    pub do_move: Move
}

pub struct Expanded {
    pub start: String,
    pub start_span: TextRange,
    pub functions: HashMap<FnSignature, Option<FnBody>>,
    pub unreachable: Vec<TextRange>
}

#[derive(Default)]
pub struct Expander {
    functions: HashMap<FnSignature, Option<FnBody>>
}
impl Expander {
    pub fn expand<R: rowan::TreeRoot<Types>>(ast: Node<R>) -> Result<Expanded> {
        let root = Root::cast(ast).expect("invalid ast");

        // Find start
        let mut starts = root.start_assignments();
        let start = starts.next().ok_or((None, GenerateError::MultipleStart))?;
        if let Some(start2) = starts.next() {
            return Err((Some(start2.node().range()), GenerateError::MultipleStart));
        }

        // Expand each path recursively, from start
        let mut expander = Self::default();
        let start_name = expander.expand_fn(&root, &start.target(), None)?;

        // Find paths not reachable from the start
        let mut unreachable = Vec::new();
        for f in root.functions() {
            if expander.functions.keys().all(|s| f.match_state().name().as_str() != &s.match_state[..s.original_len]) {
                unreachable.push(f.node().range());
            }
        }

        Ok(Expanded {
            start: start_name,
            start_span: start.node().range(),
            functions: expander.functions,
            unreachable
        })
    }
    fn expand_fn<R: rowan::TreeRoot<Types>>(
        &mut self,
        root: &Root<R>,
        invocation: &Type<R>,
        vars: Option<&HashMap<String, String>>
    ) -> Result<String> {
        let name = invocation.name();
        let name = name.as_str();
        if name == FINISH_STATE {
            return Ok(String::from(FINISH_STATE));
        }
        if let Some(f) = vars.and_then(|vars| vars.get(name)) {
            return Ok(f.to_string());
        }

        let mut functions = root.functions()
            .filter(|f| f.match_state().name().as_str() == name)
            .peekable();

        // Make sure we have at least one
        functions.peek().ok_or_else(|| (Some(invocation.node().range()), GenerateError::Unknown(name.to_string())))?;

        let mut match_state = None;

        for f in functions {
            let sign = f.match_state();
            let body = f.do_state();

            let count_sign = sign.generics().count();
            let count_invocation = invocation.generics().count();
            if count_sign != count_invocation {
                return Err((Some(invocation.node().range()), GenerateError::IllegalArguments(count_sign, count_invocation)));
            }

            // Expand all invocation arguments and bind them to their new variable names
            let mut new_vars = HashMap::new();

            let mut match_state_new = String::new();
            if match_state.is_none() {
                match_state_new.push_str(name);
            }

            for (bind, kind) in sign.generics().zip(invocation.generics()) {
                if bind.generics().next().is_some() {
                    return Err((Some(bind.node().range()), GenerateError::MatchGenerics));
                }
                let expanded = self.expand_fn(root, &kind, vars)?;
                if match_state.is_none() {
                    match_state_new.push('<');
                    match_state_new.push_str(&expanded);
                    match_state_new.push('>');
                }
                new_vars.insert(bind.name().as_str().to_string(), expanded);
            }

            if match_state.is_none() {
                match_state = Some(match_state_new);
            }

            // Expand the next state:
            let signature = FnSignature {
                match_state: match_state.clone().unwrap(),
                original_len: name.len(),
                match_input: f.match_input().value(),
            };
            if self.functions.contains_key(&signature) {
                return Ok(match_state.unwrap());
            }

            // Mark in progress, don't re-evaluate (infinite loop)
            self.functions.insert(signature.clone(), None);

            let next_name = self.expand_fn(root, &body, Some(&new_vars))?;

            self.functions.insert(signature, Some(FnBody {
                span: f.node().range(),
                do_write: f.do_write().value(),
                do_state: next_name,
                do_move: f.do_move().operation()
            }));
        }

        Ok(match_state.unwrap())
    }
}

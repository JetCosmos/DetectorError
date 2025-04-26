use std::fs;
use std::process::Command;
use swc_common::{sync::Lrc, SourceMap, GLOBALS};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};
use swc_ecma_visit::{Visit, VisitWith};
use swc_ecma_ast::*;
use swc_ecma_transforms_base::rename::renamer;
use swc_ecma_transforms::fixer;
use serde_json::Value;

struct Analyzer {
    errors: Vec<String>,
    undefined_vars: Vec<String>,
    declared_vars: Vec<String>,
}

impl Analyzer {
    fn new() -> Self {
        Analyzer {
            errors: Vec::new(),
            undefined_vars: Vec::new(),
            declared_vars: Vec::new(),
        }
    }
}

impl Visit for Analyzer {
    fn visit_ident(&mut self, ident: &Ident) {
        let name = ident.sym.to_string();
        if !self.declared_vars.contains(&name) && !self.undefined_vars.contains(&name) {
            self.undefined_vars.push(name);
        }
    }

    fn visit_var_declarator(&mut self, n: &VarDeclarator) {
        if let Pat::Ident(ident) = &n.name {
            self.declared_vars.push(ident.sym.to_string());
        }
        n.init.visit_with(self);
    }

    fn visit_call_expr(&mut self, n: &CallExpr) {
        if let Callee::Expr(expr) = &n.callee {
            if let Expr::Ident(ident) = &**expr {
                if ident.sym == "eval" {
                    self.errors.push("Uso de eval detectado, posible riesgo de seguridad".to_string());
                }
            }
        }
        n.args.visit_with(self);
    }

    fn visit_expr(&mut self, n: &Expr) {
        if let Expr::Bin(bin) = n {
            if bin.op == BinaryOp::Add {
                if let (Expr::Lit(Lit::Str(_)), Expr::Lit(Lit::Str(_))) = (&*bin.left, &*bin.right) {
                    self.errors.push("Concatenación de strings literales detectada".to_string());
                }
            }
        }
        n.visit_children_with(self);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: {} <archivo.js>", args[0]);
        std::process::exit(1);
    }
    let file_path = &args[1];
    let cm = Lrc::new(SourceMap::default());
    let code = fs::read_to_string(file_path).expect("No se pudo leer el archivo");
    let input = StringInput::new(&code, swc_common::BytePos(0), swc_common::BytePos(code.len() as u32));
    let lexer = Lexer::new(Syntax::Es(Default::default()), Default::default(), input, None);
    let mut parser = Parser::new_from(lexer);
    let module = parser.parse_module().map_err(|e| {
        eprintln!("Error de sintaxis: {:?}", e);
        std::process::exit(1);
    }).unwrap();
    let mut analyzer = Analyzer::new();
    module.visit_with(&mut analyzer);
    for err in &analyzer.errors {
        println!("Error: {}", err);
    }
    for var in &analyzer.undefined_vars {
        if !analyzer.declared_vars.contains(var) {
            println!("Advertencia: Variable no definida: {}", var);
        }
    }
    let eslint_result = Command::new("node")
        .arg("validator.js")
        .arg(file_path)
        .output()
        .expect("No se pudo ejecutar el validador");
    if !eslint_result.stdout.is_empty() {
        let eslint_output: Value = serde_json::from_slice(&eslint_result.stdout).unwrap_or_default();
        if let Some(errors) = eslint_output.as_array() {
            for err in errors {
                println!("ESLint: {} (línea {}, columna {})", 
                    err["message"].as_str().unwrap_or(""), 
                    err["line"].as_i64().unwrap_or(0), 
                    err["column"].as_i64().unwrap_or(0));
            }
        }
    }
    GLOBALS.set(&Default::default(), || {
        let mut module = module.fold_with(&mut renamer());
        module = module.fold_with(&mut fixer());
        let output_file = "obfuscated.js";
        fs::write(output_file, format!("{:?}", module)).expect("No se pudo escribir el archivo ofuscado");
        println!("Código ofuscado guardado en {}", output_file);
    });
}
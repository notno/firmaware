use actix_files::Files;
use actix_web::web;
use actix_web::{App, HttpResponse, HttpServer, Responder, get, web::Data};
use handlebars::{DirectorySourceOptions, Handlebars};
use serde::Serialize;
use std::{env, io};

#[derive(Serialize)]
struct Compliment {
    adjective: &'static str,
    verb: &'static str,
}

#[get("/")]
async fn compliment(hb: Data<Handlebars<'_>>) -> impl Responder {
    let compliment = Compliment {
        adjective: "awesome",
        verb: "is",
    };
    let html = hb.render("compliment", &compliment).unwrap();

    HttpResponse::Ok().content_type("text/html").body(html)
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let address = env::var("BIND_ADDRESS").unwrap_or_else(|_err| "localhost:8080".to_string());

    let template_service = {
        let mut handlebars = Handlebars::new();
        let mut options = DirectorySourceOptions::default();
        options.tpl_extension = "html".to_string();
        // handlebars
        //     .register_templates_directory("web\\templates", options)
        //     .map_err(|e| println!("Error registering templates: {:?}", e))
        //     .unwrap();
        if let Err(e) = handlebars.register_templates_directory("web/templates", options) {
            println!("Error registering templates: {:?}", e);
        };
        if handlebars.has_template("compliment") {
            println!("Template 'compliment' is registered.");
        } else {
            println!("Template 'compliment' is not found.");
        }


        Data::new(handlebars)
    };

    check_directory_access();
    println!("Current directory: {:?}", std::env::current_dir().unwrap());

    if !std::path::Path::new("web\\templates").exists() {
        println!("Template directory does not exist!");
    }

    println!("Starting server at http://{}", address);
    // println!("template_service: {:?}", template_service);
    let server = move || {
        App::new()
            .app_data(template_service.clone())
            .service(Files::new("/public", "web/public").show_files_listing())
            .service(compliment)
    };

    HttpServer::new(server).bind(address)?.run().await
}
use std::fs;

fn check_directory_access() {
    match fs::read_dir("web\\templates") {
        Ok(entries) => {
            println!("Directory is accessible. Contents:");
            for entry in entries {
                if let Ok(entry) = entry {
                    println!("{:?}", entry.path());
                }
            }
        }
        Err(e) => {
            println!("Failed to access directory: {:?}", e);
        }
    }
}

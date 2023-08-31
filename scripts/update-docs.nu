let file_pwd = $env.FILE_PWD
cd ($file_pwd + "/..")
rm -rf target/doc
cargo doc --no-deps --workspace
cp -r target/doc static_docs

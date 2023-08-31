# navigate to project dir
let file_pwd = $env.FILE_PWD
cd ($file_pwd + "/..")

# clear old docs
rm -rf static_docs
rm -rf target/doc

# build new docs
cargo doc --no-deps --workspace

# copy new docs over
cp -r target/doc static_docs

# include a redirect to the nlwkn lib
let redirect_content = '
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta http-equiv="refresh" content="0; url=http://www.example.com">
<title>Redirecting</title>
</head>
<body>
</body>
</html>
'
$redirect_content | save static_docs/index.html -f

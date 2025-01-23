# Creating the folder structure and the web pages for testing QUIC protocol implementation.

import os

# Base directory for the website
base_dir = "."

# Sub-pages
pages = {
    "index.html": """
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>QUIC Test Website</title>
    </head>
    <body>
        <h1>Welcome to the QUIC Test Website</h1>
        <p>Choose a page to test:</p>
        <ul>
            <li><a href="static.html">Static Page</a></li>
            <li><a href="images.html">Page with Images</a></li>
            <li><a href="dynamic.html">Page with JavaScript</a></li>
            <li><a href="form.html">Form Page</a></li>
            <li><a href="heavy.html">Heavy Content Page</a></li>
        </ul>
    </body>
    </html>
    """,
    "static.html": """
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>Static Page</title>
    </head>
    <body>
        <h1>Static Page</h1>
        <p>This is a simple static HTML page to test QUIC.</p>
    </body>
    </html>
    """,
    "images.html": """
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>Page with Images</title>
    </head>
    <body>
        <h1>Page with Images</h1>
        <p>Below are some images to test media loading over QUIC.</p>
        <img src="https://via.placeholder.com/300" alt="Placeholder Image 1">
        <img src="https://via.placeholder.com/500" alt="Placeholder Image 2">
    </body>
    </html>
    """,
    "dynamic.html": """
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>Dynamic Page</title>
        <script>
            function showAlert() {
                alert('This is a dynamic interaction!');
            }
        </script>
    </head>
    <body>
        <h1>Dynamic Page</h1>
        <button onclick="showAlert()">Click me</button>
    </body>
    </html>
    """,
    "form.html": """
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>Form Page</title>
    </head>
    <body>
        <h1>Form Page</h1>
        <form method="POST" action="#">
            <label for="name">Name:</label>
            <input type="text" id="name" name="name">
            <button type="submit">Submit</button>
        </form>
    </body>
    </html>
    """,
    "heavy.html": """
    <!DOCTYPE html>
    <html lang="en">
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>Heavy Content Page</title>
    </head>
    <body>
        <h1>Heavy Content Page</h1>
        <p>This page contains a lot of text and repeated content for testing.</p>
        """ + "<p>" * 1000 + "This is heavy content for testing QUIC." + "</p>" * 1000 + """
    </body>
    </html>
    """
}

# Create the directory and files
os.makedirs(base_dir, exist_ok=True)

for filename, content in pages.items():
    with open(os.path.join(base_dir, filename), "w", encoding="utf-8") as file:
        file.write(content)

base_dir

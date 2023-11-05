while inotifywait -e close_write content/
do
    cargo run
done

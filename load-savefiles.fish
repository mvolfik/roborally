#!/usr/bin/fish

for file in savefiles/*.bin
    echo Uploading $file
    curl -Ss "http://localhost:3000/api/savefile?game_name=%5Bloaded%20savefile%5D%20"$file -X POST --upload-file $file
    echo
end

#check if container exists first
docker run --name a2o4 -v /docker-mounts/A2O4-Server-RS/library:/home/foo/library -p 2222:22 -d atmoz/sftp foo:pass:1001

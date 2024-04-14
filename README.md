# GitHub

#### Sample Upload

```shell
curl -v -X POST \
  -H "Authorization: Bearer hello-world" \
  -H "content-location: thevickypedia/mailutils;img_4.png" \
  -F "file=@/Users/vicky/Desktop/img_4.png" \
  http://127.0.0.1:8000/upload
```

#### Sample Delete

```shell
curl -v -X DELETE \
  -H "Authorization: Bearer hello-world" \
  -H "content-location: thevickypedia/mailutils;img_4.png" \
  http://127.0.0.1:8000/delete
```

### Environment Variables

- **authorization** - Token stored in GitHub actions.
- **github_source** - Directory to store the backup.

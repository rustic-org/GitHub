# GitHub

#### Sample Upload

```shell
curl -v -X POST \
  -H "Authorization: Bearer ****" \
  -H "content-location: src/lib/template/J_Desc.png" \
  -F "file=@/Users/rustic-monkey/Desktop/J_desc.png" \
  http://127.0.0.1:8000/upload
```

#### Sample Delete

```shell
curl -v -X DELETE \
  -H "Authorization: Bearer ****" \
  -H "content-location: src/lib/template/J_Desc.png" \
  http://127.0.0.1:8000/delete
```

### Environment Variables

- **authorization** - Token stored in GitHub actions.
- **github_source** - Directory to store the backup.

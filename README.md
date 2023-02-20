## malachite ##
### Project Init 
#### Get submodule: 
##### git submodule update --init --recursive 
### Update 
#### update all submodule: 
##### git submodule update --remote --recursive

### Unitest
#### cargo tarpaulin -v


### Debugging env
#### Mac: 
##### - rust/cargo for mac
##### - brew install libelf
##### - brew install zlib 
#### Linux: 
##### - rust/cargo 
##### - apt install libelf-dev libelf1
##### - apt install zlib1g-dev

### Doc
#### cargo doc --open

### Dependency Security
#### cargo audit

### License Check
#### cargo deny check 

# kfst-rs

kfst-rs is the optional acceleration back end of kfst. If you have kfst>=4.1 installed, simply do

```bash
pip install kfst-rs
```

to install kfst-rs. It should get automatically picked up by kfst and by extension pyvoikko and pyomorfi.

To check which implementation of kfst got loaded, look at the `BACKEND` property of kfst.
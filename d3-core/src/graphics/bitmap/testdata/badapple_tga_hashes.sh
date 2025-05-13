#!/bin/bash

#!/bin/bash


#//////////
# There isn't a reason to use this script when TGAs are converted to 4444 or 1555
#/////////////

function rgba_to_argb_4444 {
    # Read from stdin, convert to hex, process with awk, and convert back to binary
    xxd -p -c 8 | awk '{
        hex = $0
        for (i = 1; i <= length(hex); i += 4) {
            byte1 = substr(hex, i, 2)
            byte2 = substr(hex, i+2, 2)

            # Extract RGBA components from nibbles
            r = strtonum("0x" substr(byte1, 1, 1))
            g = strtonum("0x" substr(byte1, 2, 1))
            b = strtonum("0x" substr(byte2, 1, 1))
            a = strtonum("0x" substr(byte2, 2, 1))

            #deaf pattern test
            #a = 13
            #r = 14
            #g = 10
            #b = 15

            # Pack into 16-bit ARGB 4444 value
            argb_4444 = (a * 4096) + (r * 256) + (g * 16) + b

            # Print as 2 bytes in hexadecimal format
            printf "%02x%02x", int(argb_4444 / 256), argb_4444 % 256
        }
    }' | xxd -r -p
}


function hex_dump {
    tee >(xxd -p >&2)
}

# Function to convert TGA to 16-bit bitmap and compute MD5 hash
tga_to_md5() {
  local input_file="$1"
  if [[ -z "$input_file" ]]; then
    echo "Usage: tga_to_md5 <input_file>"
    return 1
  fi

  if [[ ! -f "$input_file" ]]; then
    echo "File not found: $input_file"
    return 1
  fi

  # Compute the MD5 hash and store it in a variable
  local md5_hash
  md5_hash=$(convert "$input_file" -depth 4 -define quantum:format=unsigned -define quantum:bits=4 rgba:- | rgba_to_argb_4444 | md5sum | awk '{ print $1 }')

  # Print the filename and its MD5 hash
  echo "$input_file: $md5_hash"
}

# Call the function with the provided argument
tga_to_md5 "badapple.tga"
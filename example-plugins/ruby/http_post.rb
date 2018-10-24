#!/usr/local/Cellar/ruby/2.5.1/bin/ruby

require 'httparty'

resp = HTTParty.get("https://api.ipify.org")
puts resp.body

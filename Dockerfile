FROM pierrezemb/gostatic
COPY ./out /srv/http/
CMD ["-port","8080","-https-promote", "-enable-logging"]
